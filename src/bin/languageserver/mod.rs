// SPDX-License-Identifier: Apache-2.0

use forge_fmt::{format_to, parse, FormatterConfig};
use itertools::Itertools;
use num_traits::ToPrimitive;
use rust_lapper::{Interval, Lapper};
use serde_json::Value;
use solang::{
    codegen::{self, codegen, Expression},
    file_resolver::FileResolver,
    parse_and_resolve,
    sema::{
        ast::{self, RetrieveType, StructType, Type},
        builtin::{get_prototype, BUILTIN_FUNCTIONS, BUILTIN_METHODS, BUILTIN_VARIABLE},
        builtin_structs::BUILTIN_STRUCTS,
        symtable,
        tags::render,
    },
    Target,
};
use solang_parser::pt;
use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    path::PathBuf,
};
use tokio::sync::Mutex;
use tower_lsp::{
    jsonrpc::{Error, ErrorCode, Result},
    lsp_types::{
        request::{
            GotoDeclarationParams, GotoDeclarationResponse, GotoImplementationParams,
            GotoImplementationResponse, GotoTypeDefinitionParams, GotoTypeDefinitionResponse,
        },
        CompletionContext, CompletionItem, CompletionOptions, CompletionParams, CompletionResponse,
        CompletionTriggerKind, DeclarationCapability, Diagnostic, DiagnosticRelatedInformation,
        DiagnosticSeverity, DidChangeConfigurationParams, DidChangeTextDocumentParams,
        DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams,
        DidOpenTextDocumentParams, DidSaveTextDocumentParams, DocumentFormattingParams,
        ExecuteCommandOptions, ExecuteCommandParams, GotoDefinitionParams, GotoDefinitionResponse,
        Hover, HoverContents, HoverParams, HoverProviderCapability,
        ImplementationProviderCapability, InitializeParams, InitializeResult, InitializedParams,
        Location, MarkedString, MessageType, OneOf, Position, Range, ReferenceParams, RenameParams,
        ServerCapabilities, SignatureHelpOptions, TextDocumentContentChangeEvent,
        TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit,
        TypeDefinitionProviderCapability, Url, WorkspaceEdit, WorkspaceFoldersServerCapabilities,
        WorkspaceServerCapabilities,
    },
    Client, LanguageServer, LspService, Server,
};

use crate::cli::{target_arg, LanguageServerCommand};

/// Represents the type of the code object that a reference points to
/// Here "code object" refers to contracts, functions, structs, enums etc., that are defined and used within a namespace.
/// It is used along with the path of the file where the code object is defined to uniquely identify an code object.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DefinitionType {
    // function index in Namespace::functions
    Function(usize),
    // variable id
    Variable(usize),
    // (contract id where the variable is declared, variable id)
    NonLocalVariable(Option<usize>, usize),
    // user-defined struct id
    Struct(StructType),
    // (user-defined struct id, field id)
    Field(Type, usize),
    // enum index in Namespace::enums
    Enum(usize),
    // (enum index in Namespace::enums, discriminant id)
    Variant(usize, usize),
    // contract index in Namespace::contracts
    Contract(usize),
    // event index in Namespace::events
    Event(usize),
    UserType(usize),
    DynamicBytes,
}

/// Uniquely identifies a code object.
///
/// `def_type` alone does not guarantee uniqueness, i.e, there can be two or more code objects with identical `def_type`.
/// This is possible as two files can be compiled as part of different `Namespace`s and the code objects can end up having identical `def_type`.
/// For example, two structs defined in the two files can be assigned the same `def_type` - `Struct(0)` as they are both `structs` and numbers are reused across `Namespace` boundaries.
/// As it is currently possible for code objects created as part of two different `Namespace`s to be stored simultaneously in the same `SolangServer` instance,
/// in the scenario described above, code objects cannot be uniquely identified solely through `def_type`.
///
/// But `def_path` paired with `def_type` sufficiently proves uniqueness as no two code objects defined in the same file can have identical `def_type`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DefinitionIndex {
    /// stores the path of the file where the code object is mentioned in source code
    def_path: PathBuf,
    /// provides information about the type of the code object in question
    def_type: DefinitionType,
}

impl From<DefinitionType> for DefinitionIndex {
    fn from(value: DefinitionType) -> Self {
        Self {
            def_path: Default::default(),
            def_type: value,
        }
    }
}

/// Stores locations of definitions of functions, contracts, structs etc.
type Definitions = HashMap<DefinitionIndex, Range>;
/// Stores strings shown on hover
type HoverEntry = Interval<usize, String>;
/// Stores locations of function calls, uses of structs, contracts etc.
type ReferenceEntry = Interval<usize, DefinitionIndex>;
/// Stores the code objects defined within a scope and their types.
type ScopeEntry = Interval<usize, Vec<(String, Option<DefinitionIndex>)>>;
/// Stores the list of methods implemented by a contract
type Implementations = HashMap<DefinitionIndex, Vec<DefinitionIndex>>;
/// Stores types of code objects
type Types = HashMap<DefinitionIndex, DefinitionIndex>;
/// Stores all the functions that a given function overrides
type Declarations = HashMap<DefinitionIndex, Vec<DefinitionIndex>>;
/// Stores all the fields, variants, methods etc. defined for a code object
type Properties = HashMap<DefinitionIndex, HashMap<String, Option<DefinitionIndex>>>;

/// Stores information used by language server for every opened file
#[derive(Default)]
struct Files {
    caches: HashMap<PathBuf, FileCache>,
    text_buffers: HashMap<PathBuf, String>,
}

#[derive(Debug)]
struct FileCache {
    file: ast::File,
    hovers: Lapper<usize, String>,
    references: Lapper<usize, DefinitionIndex>,
    scopes: Lapper<usize, Vec<(String, Option<DefinitionIndex>)>>,
    top_level_code_objects: HashMap<String, Option<DefinitionIndex>>,
}

/// Stores information used by the language server to service requests (eg: `Go to Definitions`) received from the client.
///
/// Information stored in `GlobalCache` is extracted from the `Namespace` when the `SolangServer::build` function is run.
///
/// `GlobalCache` is global in the sense that, unlike `FileCache`, we don't have a separate instance for every file processed.
/// We have just one `GlobalCache` instance per `SolangServer` instance.
///
/// Each field stores *some information* about a code object. The code object is uniquely identified by its `DefinitionIndex`.
/// * `definitions` maps `DefinitionIndex` of a code object to its source code location where it is defined.
/// * `types` maps the `DefinitionIndex` of a code object to that of its type.
/// * `declarations` maps the `DefinitionIndex` of a `Contract` method to a list of methods that it overrides. The overridden methods belong to the parent `Contract`s
/// * `implementations` maps the `DefinitionIndex` of a `Contract` to the `DefinitionIndex`s of methods defined as part of the `Contract`.
/// * `properties` maps the `DefinitionIndex` of a code objects to the name and type of fields, variants or methods defined in the code object.
#[derive(Default)]
struct GlobalCache {
    definitions: Definitions,
    types: Types,
    declarations: Declarations,
    implementations: Implementations,
    properties: Properties,
}

impl GlobalCache {
    fn extend(&mut self, other: Self) {
        self.definitions.extend(other.definitions);
        self.types.extend(other.types);
        self.declarations.extend(other.declarations);
        self.implementations.extend(other.implementations);
        self.properties.extend(other.properties);
    }
}

// The language server currently stores some of the data grouped by the file to which the data belongs (Files struct).
// Other data (Definitions) is not grouped by file due to problems faced during cleanup,
// but is stored as a "global" field which is common to all files.
//
// Closing the file triggers the `did_close` handler function
// where we remove the entry corresponding to the closed file from Files::cache.
// If the definitions are part of `FileCache`, they are also removed with the rest of `FileCache`
// But there can be live references in other files whose definitions are defined in the closed file.
//
// Files from multiple namespaces can be open at any time in VSCode.
// But compiler currently works on the granularity of a namespace.
//
// So, we will need some way to update data that is part of the language server
// between calls to the parse_file method that provides new information for a namespace.
//
// 1. Propagate changes made to a file to all the files that depend on that.
// This requires a data structure that shows import dependencies between different files in the namespace.
// 2. Need a way to safely remove stored Definitions that are no longer used by any of the References
//
// More information can be found here: https://github.com/hyperledger-solang/solang/pull/1411
pub struct SolangServer {
    client: Client,
    target: Target,
    importpaths: Vec<PathBuf>,
    importmaps: Vec<(String, PathBuf)>,
    files: Mutex<Files>,
    global_cache: Mutex<GlobalCache>,
}

#[tokio::main(flavor = "current_thread")]
pub async fn start_server(language_args: &LanguageServerCommand) -> ! {
    let mut importpaths = Vec::new();
    let mut importmaps = Vec::new();

    if let Some(paths) = &language_args.import_path {
        for path in paths {
            importpaths.push(path.clone());
        }
    }

    if let Some(maps) = &language_args.import_map {
        for (map, path) in maps {
            importmaps.push((map.clone(), path.clone()));
        }
    }

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let target = target_arg(&language_args.target);

    let (service, socket) = LspService::new(|client| SolangServer {
        client,
        target,
        importpaths,
        importmaps,
        files: Mutex::new(Default::default()),
        global_cache: Mutex::new(Default::default()),
    });

    Server::new(stdin, stdout, socket).serve(service).await;

    std::process::exit(1);
}

impl SolangServer {
    /// Parse file
    async fn parse_file(&self, uri: Url) {
        let mut resolver = FileResolver::default();
        for (path, contents) in &self.files.lock().await.text_buffers {
            resolver.set_file_contents(path.to_str().unwrap(), contents.clone());
        }
        if let Ok(path) = uri.to_file_path() {
            let dir = path.parent().unwrap();

            resolver.add_import_path(dir);

            let mut diags = Vec::new();

            for path in &self.importpaths {
                resolver.add_import_path(path);
            }

            for (map, path) in &self.importmaps {
                resolver.add_import_map(OsString::from(map), PathBuf::from(path));
            }

            let os_str = path.file_name().unwrap();

            let mut ns = parse_and_resolve(os_str, &mut resolver, self.target);

            // codegen all the contracts; some additional errors/warnings will be detected here
            codegen(&mut ns, &Default::default());

            diags.extend(ns.diagnostics.iter().filter_map(|diag| {
                if diag.loc.file_no() != ns.top_file_no() {
                    // The first file is the one we wanted to parse; others are imported
                    return None;
                }

                let severity = match diag.level {
                    ast::Level::Info => Some(DiagnosticSeverity::INFORMATION),
                    ast::Level::Warning => Some(DiagnosticSeverity::WARNING),
                    ast::Level::Error => Some(DiagnosticSeverity::ERROR),
                    ast::Level::Debug => {
                        return None;
                    }
                };

                let related_information = if diag.notes.is_empty() {
                    None
                } else {
                    Some(
                        diag.notes
                            .iter()
                            .map(|note| DiagnosticRelatedInformation {
                                message: note.message.to_string(),
                                location: Location {
                                    uri: Url::from_file_path(&ns.files[note.loc.file_no()].path)
                                        .unwrap(),
                                    range: loc_to_range(&note.loc, &ns.files[ns.top_file_no()]),
                                },
                            })
                            .collect(),
                    )
                };

                let range = loc_to_range(&diag.loc, &ns.files[ns.top_file_no()]);

                Some(Diagnostic {
                    range,
                    message: diag.message.to_string(),
                    severity,
                    related_information,
                    ..Default::default()
                })
            }));

            let res = self.client.publish_diagnostics(uri, diags, None);

            let (file_caches, global_cache) = Builder::new(&ns).build();

            let mut files = self.files.lock().await;
            for (f, c) in ns.files.iter().zip(file_caches.into_iter()) {
                if f.cache_no.is_some() {
                    files.caches.insert(f.path.clone(), c);
                }
            }

            let mut gc = self.global_cache.lock().await;
            gc.extend(global_cache);

            res.await;
        }
    }

    /// Common code for goto_{definitions, implementations, declarations, type_definitions}
    async fn get_reference_from_params(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<DefinitionIndex>> {
        let uri = params.text_document_position_params.text_document.uri;
        let path = uri.to_file_path().map_err(|_| Error {
            code: ErrorCode::InvalidRequest,
            message: format!("Received invalid URI: {uri}").into(),
            data: None,
        })?;

        let files = self.files.lock().await;
        if let Some(cache) = files.caches.get(&path) {
            let f = &cache.file;
            if let Some(offset) = f.get_offset(
                params.text_document_position_params.position.line as _,
                params.text_document_position_params.position.character as _,
            ) {
                if let Some(reference) = cache
                    .references
                    .find(offset, offset + 1)
                    .min_by(|a, b| (a.stop - a.start).cmp(&(b.stop - b.start)))
                {
                    return Ok(Some(reference.val.clone()));
                }
            }
        }
        Ok(None)
    }
}

struct Builder<'a> {
    // `usize` is the file number that the entry belongs to
    hovers: Vec<(usize, HoverEntry)>,
    references: Vec<(usize, ReferenceEntry)>,
    scopes: Vec<(usize, ScopeEntry)>,
    top_level_code_objects: Vec<(usize, (String, Option<DefinitionIndex>))>,

    definitions: Definitions,
    types: Types,
    declarations: Declarations,
    implementations: Implementations,
    properties: Properties,

    ns: &'a ast::Namespace,
}

impl<'a> Builder<'a> {
    fn new(ns: &'a ast::Namespace) -> Self {
        Self {
            hovers: Vec::new(),
            references: Vec::new(),
            scopes: Vec::new(),
            top_level_code_objects: Vec::new(),

            definitions: HashMap::new(),
            types: HashMap::new(),
            declarations: HashMap::new(),
            implementations: HashMap::new(),
            properties: HashMap::new(),

            ns,
        }
    }

    // Constructs lookup table for the given statement by traversing the
    // statements and traversing inside the contents of the statements.
    fn statement(&mut self, stmt: &ast::Statement, symtab: &symtable::Symtable) {
        match stmt {
            ast::Statement::Block { statements, .. } => {
                for stmt in statements {
                    self.statement(stmt, symtab);
                }
            }
            ast::Statement::VariableDecl(loc, var_no, param, expr) => {
                if let Some(exp) = expr {
                    self.expression(exp, symtab);
                }

                let constant = self
                    .ns
                    .var_constants
                    .get(loc)
                    .and_then(get_constants)
                    .map(|s| format!(" = {s}"))
                    .unwrap_or_default();

                let readonly = symtab
                    .vars
                    .get(var_no)
                    .map(|var| {
                        if var.slice {
                            "\nreadonly: compiled to slice\n"
                        } else {
                            ""
                        }
                    })
                    .unwrap_or_default();

                let val = format!(
                    "{} {}{}{}",
                    param.ty.to_string(self.ns),
                    param.name_as_str(),
                    constant,
                    readonly
                );

                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: param.loc.start(),
                        stop: param.loc.exclusive_end(),
                        val: make_code_block(val),
                    },
                ));
                if let Some(id) = &param.id {
                    let file_no = id.loc.file_no();
                    let file = &self.ns.files[file_no];
                    let di = DefinitionIndex {
                        def_path: file.path.clone(),
                        def_type: DefinitionType::Variable(*var_no),
                    };
                    self.definitions
                        .insert(di.clone(), loc_to_range(&id.loc, file));
                    if let Some(dt) = get_type_definition(&param.ty) {
                        self.types.insert(di, dt.into());
                    }
                }

                if let Some(loc) = param.ty_loc {
                    if let Some(dt) = get_type_definition(&param.ty) {
                        self.references.push((
                            loc.file_no(),
                            ReferenceEntry {
                                start: loc.start(),
                                stop: loc.exclusive_end(),
                                val: dt.into(),
                            },
                        ));
                    }
                }
            }
            ast::Statement::If(_, _, expr, stat1, stat2) => {
                self.expression(expr, symtab);
                for stmt in stat1 {
                    self.statement(stmt, symtab);
                }
                for stmt in stat2 {
                    self.statement(stmt, symtab);
                }
            }
            ast::Statement::While(_, _, expr, block) => {
                self.expression(expr, symtab);
                for stmt in block {
                    self.statement(stmt, symtab);
                }
            }
            ast::Statement::For {
                init,
                cond,
                next,
                body,
                ..
            } => {
                if let Some(exp) = cond {
                    self.expression(exp, symtab);
                }
                for stat in init {
                    self.statement(stat, symtab);
                }
                if let Some(exp) = next {
                    self.expression(exp, symtab);
                }
                for stat in body {
                    self.statement(stat, symtab);
                }
            }
            ast::Statement::DoWhile(_, _, stat1, expr) => {
                self.expression(expr, symtab);
                for st1 in stat1 {
                    self.statement(st1, symtab);
                }
            }
            ast::Statement::Expression(_, _, expr) => {
                self.expression(expr, symtab);
            }
            ast::Statement::Delete(_, _, expr) => {
                self.expression(expr, symtab);
            }
            ast::Statement::Destructure(_, fields, expr) => {
                self.expression(expr, symtab);
                for field in fields {
                    match field {
                        ast::DestructureField::Expression(expr) => {
                            self.expression(expr, symtab);
                        }
                        ast::DestructureField::VariableDecl(var_no, param) => {
                            self.hovers.push((
                                param.loc.file_no(),
                                HoverEntry {
                                    start: param.loc.start(),
                                    stop: param.loc.exclusive_end(),
                                    val: self.expanded_ty(&param.ty),
                                },
                            ));
                            if let Some(id) = &param.id {
                                let file_no = id.loc.file_no();
                                let file = &self.ns.files[file_no];
                                let di = DefinitionIndex {
                                    def_path: file.path.clone(),
                                    def_type: DefinitionType::Variable(*var_no),
                                };
                                self.definitions
                                    .insert(di.clone(), loc_to_range(&id.loc, file));
                                if let Some(dt) = get_type_definition(&param.ty) {
                                    self.types.insert(di, dt.into());
                                }
                            }
                        }
                        ast::DestructureField::None => (),
                    }
                }
            }
            ast::Statement::Continue(_) => {}
            ast::Statement::Break(_) => {}
            ast::Statement::Return(_, None) => {}
            ast::Statement::Return(_, Some(expr)) => {
                self.expression(expr, symtab);
            }
            ast::Statement::Revert { args, .. } => {
                for arg in args {
                    self.expression(arg, symtab);
                }
            }
            ast::Statement::Emit {
                event_no,
                event_loc,
                args,
                ..
            } => {
                let event = &self.ns.events[*event_no];
                let mut tags = render(&event.tags);
                if !tags.is_empty() {
                    tags.push_str("\n\n");
                }
                let fields = event
                    .fields
                    .iter()
                    .map(|field| {
                        format!(
                            "\t{}{}{}",
                            field.ty.to_string(self.ns),
                            if field.indexed { " indexed " } else { " " },
                            field.name_as_str()
                        )
                    })
                    .join(",\n");
                let val = format!(
                    "event {} {{\n{}\n}}{}",
                    event.symbol_name(self.ns),
                    fields,
                    if event.anonymous { " anonymous" } else { "" }
                );
                self.hovers.push((
                    event_loc.file_no(),
                    HoverEntry {
                        start: event_loc.start(),
                        stop: event_loc.exclusive_end(),
                        val: format!("{}{}", tags, make_code_block(val)),
                    },
                ));

                self.references.push((
                    event_loc.file_no(),
                    ReferenceEntry {
                        start: event_loc.start(),
                        stop: event_loc.exclusive_end(),
                        val: DefinitionIndex {
                            def_path: Default::default(),
                            def_type: DefinitionType::Event(*event_no),
                        },
                    },
                ));

                for arg in args {
                    self.expression(arg, symtab);
                }
            }
            ast::Statement::TryCatch(_, _, try_stmt) => {
                self.expression(&try_stmt.expr, symtab);
                if let Some(clause) = try_stmt.catch_all.as_ref() {
                    for stmt in &clause.stmt {
                        self.statement(stmt, symtab);
                    }
                }
                for stmt in &try_stmt.ok_stmt {
                    self.statement(stmt, symtab);
                }
                for clause in &try_stmt.errors {
                    for stmts in &clause.stmt {
                        self.statement(stmts, symtab);
                    }
                }
            }
            ast::Statement::Underscore(_loc) => {}
            ast::Statement::Assembly(..) => {
                //unimplemented!("Assembly block not implemented in language server");
            }
        }
    }

    // Constructs lookup table by traversing the expressions and storing
    // information later used by the language server
    fn expression(&mut self, expr: &ast::Expression, symtab: &symtable::Symtable) {
        match expr {
            // Variable types expression
            ast::Expression::BoolLiteral { loc, .. } => {
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: make_code_block("bool"),
                    },
                ));
            }
            ast::Expression::BytesLiteral { loc, ty, .. } => {
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: self.expanded_ty(ty),
                    },
                ));
            }
            ast::Expression::NumberLiteral { loc, ty, value,.. } => {
                if let Type::Enum(id) = ty {
                    self.references.push((
                        loc.file_no(),
                        ReferenceEntry {
                            start: loc.start(),
                            stop: loc.exclusive_end(),
                            val: DefinitionIndex {
                                def_path: Default::default(),
                                def_type: DefinitionType::Variant(*id, value.to_u64().unwrap() as _),
                            },
                        },
                    ));
                }
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: make_code_block(ty.to_string(self.ns)),
                    }
                ));
            }
            ast::Expression::StructLiteral { id: id_path, ty, values, .. } => {
                if let Type::Struct(StructType::UserDefined(id)) = ty {
                    let loc = id_path.identifiers.last().unwrap().loc;
                    self.references.push((
                        loc.file_no(),
                        ReferenceEntry {
                            start: loc.start(),
                            stop: loc.exclusive_end(),
                            val: DefinitionIndex {
                                def_path: Default::default(),
                                def_type: DefinitionType::Struct(StructType::UserDefined(*id)),
                            },
                        },
                    ));
                }

                for (i, (field_name, expr)) in values.iter().enumerate() {
                    self.expression(expr, symtab);

                    if let Some(pt::Identifier { loc: field_name_loc, ..}) = field_name {
                        self.references.push((
                            field_name_loc.file_no(),
                            ReferenceEntry {
                                start: field_name_loc.start(),
                                stop: field_name_loc.exclusive_end(),
                                val: DefinitionIndex {
                                    def_path: Default::default(),
                                    def_type: DefinitionType::Field(ty.clone(), i),
                                },
                            },
                        ));
                    }
                }
            }
            ast::Expression::ArrayLiteral { values, .. }
            | ast::Expression::ConstArrayLiteral { values, .. } => {
                for expr in values {
                    self.expression(expr, symtab);
                }
            }

            // Arithmetic expression
            ast::Expression::Add {
                loc,
                ty,
                unchecked,
                left,
                right,
            } => {
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: format!(
                            "{} {} addition",
                            if *unchecked { "unchecked " } else { "" },
                            ty.to_string(self.ns)
                        ),
                    },
                ));

                self.expression(left, symtab);
                self.expression(right, symtab);
            }
            ast::Expression::Subtract {
                loc,
                ty,
                unchecked,
                left,
                right,
            } => {
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: format!(
                            "{} {} subtraction",
                            if *unchecked { "unchecked " } else { "" },
                            ty.to_string(self.ns)
                        ),
                    }
                ));

                self.expression(left, symtab);
                self.expression(right, symtab);
            }
            ast::Expression::Multiply {
                loc,
                ty,
                unchecked,
                left,
                right,
            } => {
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: format!(
                            "{} {} multiply",
                            if *unchecked { "unchecked " } else { "" },
                            ty.to_string(self.ns)
                        ),
                    },
                ));

                self.expression(left, symtab);
                self.expression(right, symtab);
            }
            ast::Expression::Divide {
                loc,
                ty,
                left,
                right,
            } => {
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: format!("{} divide", ty.to_string(self.ns)),
                    },
                ));

                self.expression(left, symtab);
                self.expression(right, symtab);
            }
            ast::Expression::Modulo {
                loc,
                ty,
                left,
                right,
            } => {
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: format!("{} modulo", ty.to_string(self.ns)),
                    },
                ));

                self.expression(left, symtab);
                self.expression(right, symtab);
            }
            ast::Expression::Power {
                loc,
                ty,
                unchecked,
                base,
                exp,
            } => {
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: format!(
                            "{} {}power",
                            if *unchecked { "unchecked " } else { "" },
                            ty.to_string(self.ns)
                        ),
                    },
                ));

                self.expression(base, symtab);
                self.expression(exp, symtab);
            }

            // Bitwise expresion
            ast::Expression::BitwiseOr { left, right, .. }
            | ast::Expression::BitwiseAnd { left, right, .. }
            | ast::Expression::BitwiseXor { left, right, .. }
            | ast::Expression::ShiftLeft { left, right, .. }
            | ast::Expression::ShiftRight { left, right, .. }
            // Logical expression
            | ast::Expression::Or { left, right, .. }
            | ast::Expression::And { left, right, .. }
            // Compare expression
            | ast::Expression::Equal { left, right, .. }
            | ast::Expression::More { left, right, .. }
            | ast::Expression::MoreEqual { left, right, .. }
            | ast::Expression::Less { left, right, .. }
            | ast::Expression::LessEqual { left, right, .. }
            | ast::Expression::NotEqual { left, right, .. }
            // assign
            | ast::Expression::Assign { left, right, .. }
                        => {
                self.expression(left, symtab);
                self.expression(right, symtab);
            }

            // Variable expression
            ast::Expression::Variable { loc, ty, var_no } => {
                let name = if let Some(var) = symtab.vars.get(var_no) {
                    &var.id.name
                } else {
                    ""
                };
                let readonly = symtab
                    .vars
                    .get(var_no)
                    .map(|var| {
                        if var.slice {
                            "\nreadonly: compiled to slice\n"
                        } else {
                            ""
                        }
                    })
                    .unwrap_or_default();

                let val = format!("{} {}{}", ty.to_string(self.ns), name, readonly);

                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: make_code_block(val),
                    },
                ));

                self.references.push((
                    loc.file_no(),
                    ReferenceEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: DefinitionIndex {
                            def_path: Default::default(),
                            def_type: DefinitionType::Variable(*var_no),
                        },
                    },
                ));
            }
            ast::Expression::ConstantVariable { loc, ty, contract_no, var_no } => {
                let (contract, name) = if let Some(contract_no) = contract_no {
                    let contract = format!("{}.", self.ns.contracts[*contract_no].id);
                    let name = &self.ns.contracts[*contract_no].variables[*var_no].name;
                    (contract, name)
                } else {
                    let contract = String::new();
                    let name = &self.ns.constants[*var_no].name;
                    (contract, name)
                };
                let constant = self
                    .ns
                    .var_constants
                    .get(loc)
                    .and_then(get_constants)
                    .map(|s| format!(" = {s}"))
                    .unwrap_or_default();
                let val = format!("{} constant {}{}{}", ty.to_string(self.ns), contract, name, constant);
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: make_code_block(val),
                    },
                ));
                self.references.push((
                    loc.file_no(),
                    ReferenceEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: DefinitionIndex {
                            def_path: Default::default(),
                            def_type: DefinitionType::NonLocalVariable(*contract_no, *var_no),
                        },
                    },
                ));
            }
            ast::Expression::StorageVariable { loc, ty, contract_no, var_no } => {
                let contract = &self.ns.contracts[*contract_no];
                let name = &contract.variables[*var_no].name;
                let val = format!("{} {}.{}", ty.to_string(self.ns), contract.id, name);
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: make_code_block(val),
                    },
                ));
                self.references.push((
                    loc.file_no(),
                    ReferenceEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: DefinitionIndex {
                            def_path: Default::default(),
                            def_type: DefinitionType::NonLocalVariable(Some(*contract_no), *var_no),
                        },
                    },
                ));
            }
            // Load expression
            ast::Expression::Load { expr, .. }
            | ast::Expression::StorageLoad { expr, .. }
            | ast::Expression::ZeroExt { expr, .. }
            | ast::Expression::SignExt { expr, .. }
            | ast::Expression::Trunc { expr, .. }
            | ast::Expression::Cast { expr, .. }
            | ast::Expression::BytesCast { expr, .. }
            // Increment-Decrement expression
            | ast::Expression::PreIncrement { expr, .. }
            | ast::Expression::PreDecrement { expr, .. }
            | ast::Expression::PostIncrement { expr, .. }
            | ast::Expression::PostDecrement { expr, .. }
            // Other Unary
            | ast::Expression::Not { expr, .. }
            | ast::Expression::BitwiseNot { expr, .. }
            | ast::Expression::Negate { expr, .. } => {
                self.expression(expr, symtab);
            }

            ast::Expression::ConditionalOperator {
                cond,
                true_option: left,
                false_option: right,
                ..
            } => {
                self.expression(cond, symtab);
                self.expression(left, symtab);
                self.expression(right, symtab);
            }

            ast::Expression::Subscript { array, index, .. } => {
                self.expression(array, symtab);
                self.expression(index, symtab);
            }

            ast::Expression::StructMember {  loc, expr, field, ty } => {
                self.expression(expr, symtab);

                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: make_code_block(ty.to_string(self.ns)),
                    },
                ));

                let t = expr.ty().deref_any().clone();
                if let Type::Struct(StructType::UserDefined(_)) = t {
                    self.references.push((
                        loc.file_no(),
                        ReferenceEntry {
                            start: loc.start(),
                            stop: loc.exclusive_end(),
                            val: DefinitionIndex {
                                def_path: Default::default(),
                                def_type: DefinitionType::Field(t, *field),
                            },
                        },
                    ));
                }
            }

            // Array operation expression
            ast::Expression::AllocDynamicBytes { loc, ty, length,  .. } => {
                if let Some(dt) = get_type_definition(ty) {
                    self.references.push((
                        loc.file_no(),
                        ReferenceEntry {
                            start: loc.start(),
                            stop: loc.exclusive_end(),
                            val: dt.into(),
                        },
                    ));
                }
                self.expression(length, symtab);
            }
            ast::Expression::StorageArrayLength { array, .. } => {
                self.expression(array, symtab);
            }

            // String operations expression
            ast::Expression::StringCompare { left, right, .. } => {
                if let ast::StringLocation::RunTime(expr) = left {
                    self.expression(expr, symtab);
                }
                if let ast::StringLocation::RunTime(expr) = right {
                    self.expression(expr, symtab);
                }
            }

            ast::Expression::InternalFunction {id, function_no, ..} => {
                let fnc = &self.ns.functions[*function_no];
                let mut msg_tg = render(&fnc.tags[..]);
                if !msg_tg.is_empty() {
                    msg_tg.push_str("\n\n");
                }

                let params = fnc.params.iter().map(|parm| format!("{} {}", parm.ty.to_string(self.ns), parm.name_as_str())).join(", ");

                let rets = fnc.returns.iter().map(|ret| {
                        let mut msg = ret.ty.to_string(self.ns);
                        if ret.name_as_str() != "" {
                            msg = format!("{} {}", msg, ret.name_as_str());
                        }
                        msg
                    }).join(", ");

                let contract = fnc.contract_no.map(|contract_no| format!("{}.", self.ns.contracts[contract_no].id)).unwrap_or_default();

                let val = format!("{} {}{}({}) returns ({})\n", fnc.ty, contract, fnc.id, params, rets);

                let func_loc = id.identifiers.last().unwrap().loc;

                self.hovers.push((
                    func_loc.file_no(),
                    HoverEntry {
                        start: func_loc.start(),
                        stop: func_loc.exclusive_end(),
                        val: format!("{}{}", msg_tg, make_code_block(val)),
                    },
                ));
                self.references.push((
                    func_loc.file_no(),
                    ReferenceEntry {
                        start: func_loc.start(),
                        stop: func_loc.exclusive_end(),
                        val: DefinitionIndex {
                            def_path: Default::default(),
                            def_type: DefinitionType::Function(*function_no),
                        },
                    },
                ));
            }

            // Function call expression
            ast::Expression::InternalFunctionCall {
                function,
                args,
                ..
            } => {
                if let ast::Expression::InternalFunction { .. } = function.as_ref() {
                    self.expression(function, symtab);
                }

                for arg in args {
                    self.expression(arg, symtab);
                }
            }

            ast::Expression::ExternalFunction { loc, address, function_no, .. } => {
                // modifiers do not have mutability, bases or modifiers themselves
                let fnc = &self.ns.functions[*function_no];
                let mut msg_tg = render(&fnc.tags[..]);
                if !msg_tg.is_empty() {
                    msg_tg.push_str("\n\n");
                }

                let params = fnc.params.iter().map(|parm| format!("{} {}", parm.ty.to_string(self.ns), parm.name_as_str())).join(", ");

                let rets = fnc.returns.iter().map(|ret| {
                        let mut msg = ret.ty.to_string(self.ns);
                        if ret.name_as_str() != "" {
                            msg = format!("{} {}", msg, ret.name_as_str());
                        }
                        msg
                    }).join(", ");

                let contract = fnc.contract_no.map(|contract_no| format!("{}.", self.ns.contracts[contract_no].id)).unwrap_or_default();

                let val = format!("{} {}{}({}) returns ({})\n", fnc.ty, contract, fnc.id, params, rets);

                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: format!("{}{}", msg_tg, make_code_block(val)),
                    },
                ));
                self.references.push((
                    loc.file_no(),
                    ReferenceEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: DefinitionIndex {
                            def_path: Default::default(),
                            def_type: DefinitionType::Function(*function_no),
                        },
                    },
                ));

                self.expression(address, symtab);
            }

            ast::Expression::ExternalFunctionCall {
                function,
                args,
                call_args,
                ..
            } => {
                if let ast::Expression::ExternalFunction { .. } = function.as_ref() {
                    self.expression(function, symtab);
                }
                for arg in args {
                    self.expression(arg, symtab);
                }
                if let Some(value) = &call_args.value {
                    self.expression(value, symtab);
                }
                if let Some(gas) = &call_args.gas {
                    self.expression(gas, symtab);
                }
            }
            ast::Expression::ExternalFunctionCallRaw {
                address,
                args,
                call_args,
                ..
            } => {
                self.expression(args, symtab);
                self.expression(address, symtab);
                if let Some(value) = &call_args.value {
                    self.expression(value, symtab);
                }
                if let Some(gas) = &call_args.gas {
                    self.expression(gas, symtab);
                }
            }
            ast::Expression::Constructor {
                args, call_args, ..
            } => {
                if let Some(gas) = &call_args.gas {
                    self.expression(gas, symtab);
                }
                for arg in args {
                    self.expression(arg, symtab);
                }
                if let Some(optval) = &call_args.value {
                    self.expression(optval, symtab);
                }
                if let Some(optsalt) = &call_args.salt {
                    self.expression(optsalt, symtab);
                }
                if let Some(seeds) = &call_args.seeds {
                    self.expression(seeds, symtab);
                }
            }
            ast::Expression::Builtin { loc, kind, args, .. } => {
                let (rets, name, params, doc) = if let Some(protval) = get_prototype(*kind) {
                    let rets = protval.ret.iter().map(|ret| ret.to_string(self.ns)).join(" ");

                    let mut params = protval.params.iter().map(|param| param.to_string(self.ns)).join(" ");

                    if !params.is_empty() {
                        params = format!("({params})");
                    }
                    let doc = format!("{}\n\n", protval.doc);
                    (rets, protval.name, params, doc)
                } else {
                    (String::new(), "", String::new(), String::new())
                };

                let val = make_code_block(format!("[built-in] {rets} {name} {params}"));
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: format!("{doc}{val}"),
                    },
                ));

                for expr in args {
                    self.expression(expr, symtab);
                }
            }
            ast::Expression::FormatString {format, .. } => {
                for (_, e) in format {
                    self.expression(e, symtab);
                }
            }
            ast::Expression::List {  list, .. } => {
                for expr in list {
                    self.expression(expr, symtab);
                }
            }
            _ => {}
        }
    }

    // Constructs contract fields and stores it in the lookup table.
    fn contract_variable(
        &mut self,
        variable: &ast::Variable,
        symtab: &symtable::Symtable,
        contract_no: Option<usize>,
        var_no: usize,
    ) {
        let mut tags = render(&variable.tags[..]);
        if !tags.is_empty() {
            tags.push_str("\n\n");
        }
        let val = make_code_block(format!(
            "{} {}",
            variable.ty.to_string(self.ns),
            variable.name
        ));

        if let Some(expr) = &variable.initializer {
            self.expression(expr, symtab);
        }

        let file_no = variable.loc.file_no();
        let file = &self.ns.files[file_no];
        self.hovers.push((
            file_no,
            HoverEntry {
                start: variable.loc.start(),
                stop: variable.loc.start() + variable.name.len(),
                val: format!("{tags}{val}"),
            },
        ));

        let di = DefinitionIndex {
            def_path: file.path.clone(),
            def_type: DefinitionType::NonLocalVariable(contract_no, var_no),
        };
        self.definitions
            .insert(di.clone(), loc_to_range(&variable.loc, file));
        if let Some(dt) = get_type_definition(&variable.ty) {
            self.types.insert(di, dt.into());
        }

        if contract_no.is_none() {
            self.top_level_code_objects.push((
                file_no,
                (
                    variable.name.clone(),
                    get_type_definition(&variable.ty).map(|dt| dt.into()),
                ),
            ))
        }
    }

    // Constructs struct fields and stores it in the lookup table.
    fn field(&mut self, id: usize, field_id: usize, field: &ast::Parameter<Type>) {
        if let Some(loc) = field.ty_loc {
            if let Some(dt) = get_type_definition(&field.ty) {
                self.references.push((
                    loc.file_no(),
                    ReferenceEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: dt.into(),
                    },
                ));
            }
        }
        let loc = field.id.as_ref().map(|id| &id.loc).unwrap_or(&field.loc);
        let file_no = loc.file_no();
        let file = &self.ns.files[file_no];
        self.hovers.push((
            file_no,
            HoverEntry {
                start: loc.start(),
                stop: loc.exclusive_end(),
                val: make_code_block(format!(
                    "{} {}",
                    field.ty.to_string(self.ns),
                    field.name_as_str()
                )),
            },
        ));

        let di = DefinitionIndex {
            def_path: file.path.clone(),
            def_type: DefinitionType::Field(
                Type::Struct(ast::StructType::UserDefined(id)),
                field_id,
            ),
        };
        self.definitions.insert(di.clone(), loc_to_range(loc, file));
        if let Some(dt) = get_type_definition(&field.ty) {
            self.types.insert(di, dt.into());
        }
    }

    /// Traverses namespace to extract information used later by the language server
    /// This includes hover messages, locations where code objects are declared and used
    fn build(mut self) -> (Vec<FileCache>, GlobalCache) {
        for (ei, enum_decl) in self.ns.enums.iter().enumerate() {
            for (discriminant, (nam, loc)) in enum_decl.values.iter().enumerate() {
                let file_no = loc.file_no();
                let file = &self.ns.files[file_no];
                self.hovers.push((
                    file_no,
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: make_code_block(format!(
                            "enum {}.{} {}",
                            enum_decl.id, nam, discriminant
                        )),
                    },
                ));

                let di = DefinitionIndex {
                    def_path: file.path.clone(),
                    def_type: DefinitionType::Variant(ei, discriminant),
                };
                self.definitions.insert(di.clone(), loc_to_range(loc, file));

                let dt = DefinitionType::Enum(ei);
                self.types.insert(di, dt.into());
            }

            let file_no = enum_decl.id.loc.file_no();
            let file = &self.ns.files[file_no];
            self.hovers.push((
                file_no,
                HoverEntry {
                    start: enum_decl.id.loc.start(),
                    stop: enum_decl.id.loc.exclusive_end(),
                    val: render(&enum_decl.tags[..]),
                },
            ));

            let def_index = DefinitionIndex {
                def_path: file.path.clone(),
                def_type: DefinitionType::Enum(ei),
            };
            self.definitions
                .insert(def_index.clone(), loc_to_range(&enum_decl.id.loc, file));

            self.properties.insert(
                def_index.clone(),
                enum_decl
                    .values
                    .iter()
                    .map(|(name, _)| (name.clone(), None))
                    .collect(),
            );

            if enum_decl.contract.is_none() {
                self.top_level_code_objects
                    .push((file_no, (enum_decl.id.name.clone(), Some(def_index))));
            }
        }

        for (si, struct_decl) in self.ns.structs.iter().enumerate() {
            if matches!(struct_decl.loc, pt::Loc::File(_, _, _)) {
                for (fi, field) in struct_decl.fields.iter().enumerate() {
                    self.field(si, fi, field);
                }

                let file_no = struct_decl.id.loc.file_no();
                let file = &self.ns.files[file_no];
                self.hovers.push((
                    file_no,
                    HoverEntry {
                        start: struct_decl.id.loc.start(),
                        stop: struct_decl.id.loc.exclusive_end(),
                        val: render(&struct_decl.tags[..]),
                    },
                ));

                let def_index = DefinitionIndex {
                    def_path: file.path.clone(),
                    def_type: DefinitionType::Struct(StructType::UserDefined(si)),
                };
                self.definitions
                    .insert(def_index.clone(), loc_to_range(&struct_decl.id.loc, file));

                self.properties.insert(
                    def_index.clone(),
                    struct_decl
                        .fields
                        .iter()
                        .filter_map(|field| {
                            let def_index =
                                get_type_definition(&field.ty).map(|def_type| DefinitionIndex {
                                    def_path: file.path.clone(),
                                    def_type,
                                });
                            field.id.as_ref().map(|id| (id.name.clone(), def_index))
                        })
                        .collect(),
                );

                if struct_decl.contract.is_none() {
                    self.top_level_code_objects
                        .push((file_no, (struct_decl.id.name.clone(), Some(def_index))));
                }
            }
        }

        for (i, func) in self.ns.functions.iter().enumerate() {
            if func.is_accessor || func.loc == pt::Loc::Builtin {
                // accessor functions are synthetic; ignore them, all the locations are fake
                continue;
            }

            if let Some(bump) = &func.annotations.bump {
                self.expression(&bump.1, &func.symtable);
            }

            for seed in &func.annotations.seeds {
                self.expression(&seed.1, &func.symtable);
            }

            if let Some(space) = &func.annotations.space {
                self.expression(&space.1, &func.symtable);
            }

            if let Some((loc, name)) = &func.annotations.payer {
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: format!("payer account: {name}"),
                    },
                ));
            }

            for (i, param) in func.params.iter().enumerate() {
                let loc = param.id.as_ref().map(|id| &id.loc).unwrap_or(&param.loc);
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: self.expanded_ty(&param.ty),
                    },
                ));

                if let Some(Some(var_no)) = func.symtable.arguments.get(i) {
                    if let Some(id) = &param.id {
                        let file_no = id.loc.file_no();
                        let file = &self.ns.files[file_no];
                        let di = DefinitionIndex {
                            def_path: file.path.clone(),
                            def_type: DefinitionType::Variable(*var_no),
                        };
                        self.definitions
                            .insert(di.clone(), loc_to_range(&id.loc, file));
                        if let Some(dt) = get_type_definition(&param.ty) {
                            self.types.insert(di, dt.into());
                        }
                    }
                }

                if let Some(ty_loc) = param.ty_loc {
                    if let Some(dt) = get_type_definition(&param.ty) {
                        self.references.push((
                            ty_loc.file_no(),
                            ReferenceEntry {
                                start: ty_loc.start(),
                                stop: ty_loc.exclusive_end(),
                                val: dt.into(),
                            },
                        ));
                    }
                }
            }

            for (i, ret) in func.returns.iter().enumerate() {
                let loc = ret.id.as_ref().map(|id| &id.loc).unwrap_or(&ret.loc);
                self.hovers.push((
                    loc.file_no(),
                    HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: self.expanded_ty(&ret.ty),
                    },
                ));

                if let Some(id) = &ret.id {
                    if let Some(var_no) = func.symtable.returns.get(i) {
                        let file_no = id.loc.file_no();
                        let file = &self.ns.files[file_no];
                        let di = DefinitionIndex {
                            def_path: file.path.clone(),
                            def_type: DefinitionType::Variable(*var_no),
                        };
                        self.definitions
                            .insert(di.clone(), loc_to_range(&id.loc, file));
                        if let Some(dt) = get_type_definition(&ret.ty) {
                            self.types.insert(di, dt.into());
                        }
                    }
                }

                if let Some(ty_loc) = ret.ty_loc {
                    if let Some(dt) = get_type_definition(&ret.ty) {
                        self.references.push((
                            ty_loc.file_no(),
                            ReferenceEntry {
                                start: ty_loc.start(),
                                stop: ty_loc.exclusive_end(),
                                val: dt.into(),
                            },
                        ));
                    }
                }
            }

            for stmt in &func.body {
                self.statement(stmt, &func.symtable);
            }

            let file_no = func.id.loc.file_no();
            let file = &self.ns.files[file_no];
            self.definitions.insert(
                DefinitionIndex {
                    def_path: file.path.clone(),
                    def_type: DefinitionType::Function(i),
                },
                loc_to_range(&func.id.loc, file),
            );

            self.scopes.extend(func.symtable.scopes.iter().map(|scope| {
                let loc = scope.loc.unwrap();
                let scope_entry = ScopeEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: scope
                        .names
                        .values()
                        .filter_map(|pos| {
                            func.symtable.vars.get(pos).map(|var| {
                                (
                                    var.id.name.clone(),
                                    get_type_definition(&var.ty).map(|def_type| def_type.into()),
                                )
                            })
                        })
                        .collect_vec(),
                };
                (file_no, scope_entry)
            }));

            if func.contract_no.is_none() {
                self.top_level_code_objects
                    .push((file_no, (func.id.name.clone(), None)))
            }
        }

        for (i, constant) in self.ns.constants.iter().enumerate() {
            let samptb = symtable::Symtable::default();
            self.contract_variable(constant, &samptb, None, i);
        }

        for (ci, contract) in self.ns.contracts.iter().enumerate() {
            for base in &contract.bases {
                let file_no = base.loc.file_no();
                self.hovers.push((
                    file_no,
                    HoverEntry {
                        start: base.loc.start(),
                        stop: base.loc.exclusive_end(),
                        val: make_code_block(format!(
                            "contract {}",
                            self.ns.contracts[base.contract_no].id
                        )),
                    },
                ));
                self.references.push((
                    file_no,
                    ReferenceEntry {
                        start: base.loc.start(),
                        stop: base.loc.exclusive_end(),
                        val: DefinitionIndex {
                            def_path: Default::default(),
                            def_type: DefinitionType::Contract(base.contract_no),
                        },
                    },
                ));
            }

            for (i, variable) in contract.variables.iter().enumerate() {
                let symtable = symtable::Symtable::default();
                self.contract_variable(variable, &symtable, Some(ci), i);
            }

            let file_no = contract.loc.file_no();
            let file = &self.ns.files[file_no];
            self.hovers.push((
                file_no,
                HoverEntry {
                    start: contract.id.loc.start(),
                    stop: contract.id.loc.exclusive_end(),
                    val: render(&contract.tags[..]),
                },
            ));

            let contract_def_index = DefinitionIndex {
                def_path: file.path.clone(),
                def_type: DefinitionType::Contract(ci),
            };

            self.definitions.insert(
                contract_def_index.clone(),
                loc_to_range(&contract.id.loc, file),
            );

            let impls = contract
                .functions
                .iter()
                .map(|f| DefinitionIndex {
                    def_path: file.path.clone(), // all the implementations for a contract are present in the same file in solidity
                    def_type: DefinitionType::Function(*f),
                })
                .collect();

            self.implementations
                .insert(contract_def_index.clone(), impls);

            let decls = contract
                .virtual_functions
                .iter()
                .filter_map(|(_, indices)| {
                    // due to the way the `indices` vector is populated during namespace creation,
                    // the last element in the vector contains the overriding function that belongs to the current contract.
                    let func = DefinitionIndex {
                        def_path: file.path.clone(),
                        // `unwrap` is alright here as the `indices` vector is guaranteed to have at least 1 element
                        // the vector is always initialised with one initial element
                        // and the elements in the vector are never removed during namespace construction
                        def_type: DefinitionType::Function(indices.last().copied().unwrap()),
                    };

                    // get all the functions overridden by the current function
                    let all_decls: HashSet<usize> = HashSet::from_iter(indices.iter().copied());

                    // choose the overridden functions that belong to the parent contracts
                    // due to multiple inheritance, a contract can have multiple parents
                    let parent_decls = contract
                        .bases
                        .iter()
                        .map(|b| {
                            let p = &self.ns.contracts[b.contract_no];
                            HashSet::from_iter(p.functions.iter().copied())
                                .intersection(&all_decls)
                                .copied()
                                .collect::<HashSet<usize>>()
                        })
                        .reduce(|acc, e| acc.union(&e).copied().collect());

                    // get the `DefinitionIndex`s of the overridden functions
                    parent_decls.map(|parent_decls| {
                        let decls = parent_decls
                            .iter()
                            .map(|&i| {
                                let loc = self.ns.functions[i].loc;
                                DefinitionIndex {
                                    def_path: self.ns.files[loc.file_no()].path.clone(),
                                    def_type: DefinitionType::Function(i),
                                }
                            })
                            .collect::<Vec<_>>();

                        (func, decls)
                    })
                });

            self.declarations.extend(decls);

            // Code objects defined within the contract

            let functions = contract.functions.iter().filter_map(|&fno| {
                self.ns
                    .functions
                    .get(fno)
                    .map(|func| (func.id.name.clone(), None))
            });

            let structs = self
                .ns
                .structs
                .iter()
                .enumerate()
                .filter_map(|(i, r#struct)| match &r#struct.contract {
                    Some(contract_name) if contract_name == &contract.id.name => Some((
                        r#struct.id.name.clone(),
                        Some(DefinitionType::Struct(StructType::UserDefined(i))),
                    )),
                    _ => None,
                });

            let enums =
                self.ns
                    .enums
                    .iter()
                    .enumerate()
                    .filter_map(|(i, r#enum)| match &r#enum.contract {
                        Some(contract_name) if contract_name == &contract.id.name => {
                            Some((r#enum.id.name.clone(), Some(DefinitionType::Enum(i))))
                        }
                        _ => None,
                    });

            let events =
                self.ns
                    .events
                    .iter()
                    .enumerate()
                    .filter_map(|(i, event)| match &event.contract {
                        Some(event_contract) if *event_contract == ci => {
                            Some((event.id.name.clone(), Some(DefinitionType::Event(i))))
                        }
                        _ => None,
                    });

            let variables = contract.variables.iter().map(|var| {
                (
                    var.name.clone(),
                    get_type_definition(&var.ty).map(|def_type| def_type.into()),
                )
            });

            let contract_contents = functions
                .chain(structs)
                .chain(enums)
                .chain(events)
                .map(|(name, dt)| {
                    let def_index = dt.map(|def_type| DefinitionIndex {
                        def_path: file.path.clone(),
                        def_type,
                    });
                    (name, def_index)
                })
                .chain(variables);

            self.properties.insert(
                contract_def_index.clone(),
                contract_contents.clone().collect(),
            );

            self.scopes.push((
                file_no,
                ScopeEntry {
                    start: contract.loc.start(),
                    stop: contract.loc.exclusive_end(),
                    val: contract_contents.collect(),
                },
            ));

            // Contracts can't be defined within other contracts.
            // So all the contracts are top level objects in a file.
            self.top_level_code_objects.push((
                file_no,
                (contract.id.name.clone(), Some(contract_def_index)),
            ));
        }

        for (ei, event) in self.ns.events.iter().enumerate() {
            for (fi, field) in event.fields.iter().enumerate() {
                self.field(ei, fi, field);
            }

            let file_no = event.id.loc.file_no();
            let file = &self.ns.files[file_no];
            self.hovers.push((
                file_no,
                HoverEntry {
                    start: event.id.loc.start(),
                    stop: event.id.loc.exclusive_end(),
                    val: render(&event.tags[..]),
                },
            ));

            let def_index = DefinitionIndex {
                def_path: file.path.clone(),
                def_type: DefinitionType::Event(ei),
            };
            self.definitions
                .insert(def_index.clone(), loc_to_range(&event.id.loc, file));

            if event.contract.is_none() {
                self.top_level_code_objects
                    .push((file_no, (event.id.name.clone(), Some(def_index))));
            }
        }

        for lookup in &mut self.hovers {
            if let Some(msg) =
                self.ns
                    .hover_overrides
                    .get(&pt::Loc::File(lookup.0, lookup.1.start, lookup.1.stop))
            {
                lookup.1.val.clone_from(msg);
            }
        }

        // `defs_to_files` and `defs_to_file_nos` are used to insert the correct filepath where a code object is defined.
        // previously, a dummy path was filled.
        // In a single namespace, there can't be two (or more) code objects with a given `DefinitionType`.
        // So, there exists a one-to-one mapping between `DefinitionIndex` and `DefinitionType` when we are dealing with just one namespace.
        let defs_to_files = self
            .definitions
            .keys()
            .map(|key| (key.def_type.clone(), key.def_path.clone()))
            .collect::<HashMap<DefinitionType, PathBuf>>();

        let defs_to_file_nos = self
            .ns
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| (f.path.clone(), i))
            .collect::<HashMap<PathBuf, usize>>();

        for val in self.types.values_mut() {
            if let Some(path) = defs_to_files.get(&val.def_type) {
                val.def_path.clone_from(path);
            }
        }

        for (di, range) in &self.definitions {
            if let Some(&file_no) = defs_to_file_nos.get(&di.def_path) {
                let file = &self.ns.files[file_no];
                self.references.push((
                    file_no,
                    ReferenceEntry {
                        start: file
                            .get_offset(range.start.line as usize, range.start.character as usize)
                            .unwrap(),
                        // 1 is added to account for the fact that `Lapper` expects half open ranges of the type:  [`start`, `stop`)
                        // i.e, `start` included but `stop` excluded.
                        stop: file
                            .get_offset(range.end.line as usize, range.end.character as usize)
                            .unwrap()
                            + 1,
                        val: di.clone(),
                    },
                ));
            }
        }

        let file_caches = self
            .ns
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| FileCache {
                file: f.clone(),
                // get `hovers` that belong to the current file
                hovers: Lapper::new(
                    self.hovers
                        .iter()
                        .filter(|h| h.0 == i)
                        .map(|(_, i)| i.clone())
                        .collect(),
                ),
                // get `references` that belong to the current file
                references: Lapper::new(
                    self.references
                        .iter()
                        .filter(|reference| reference.0 == i)
                        .map(|(_, i)| {
                            let mut i = i.clone();
                            if let Some(def_path) = defs_to_files.get(&i.val.def_type) {
                                i.val.def_path.clone_from(def_path);
                            }
                            i
                        })
                        .collect(),
                ),
                scopes: Lapper::new(
                    self.scopes
                        .iter()
                        .filter(|scope| scope.0 == i)
                        .map(|(_, scope)| {
                            let mut scope = scope.clone();
                            for val in &mut scope.val {
                                if let Some(val) = &mut val.1 {
                                    if let Some(def_path) = defs_to_files.get(&val.def_type) {
                                        val.def_path.clone_from(def_path);
                                    }
                                }
                            }
                            scope
                        })
                        .collect(),
                ),
                top_level_code_objects: self
                    .top_level_code_objects
                    .iter_mut()
                    .filter(|code_object| code_object.0 == i)
                    .map(|code_object| {
                        if let Some(DefinitionIndex { def_path, def_type }) = &mut code_object.1 .1
                        {
                            if def_path.to_str().unwrap() == "" {
                                if let Some(dp) = defs_to_files.get(def_type) {
                                    def_path.clone_from(dp);
                                }
                            }
                        }
                        code_object.1.clone()
                    })
                    .collect(),
            })
            .collect();

        for properties in self.properties.values_mut() {
            for def_index in properties.values_mut().flatten() {
                if def_index.def_path.to_str().unwrap() == "" {
                    if let Some(def_path) = defs_to_files.get(&def_index.def_type) {
                        def_index.def_path.clone_from(def_path);
                    }
                }
            }
        }

        let global_cache = GlobalCache {
            definitions: self.definitions,
            types: self.types,
            declarations: self.declarations,
            implementations: self.implementations,
            properties: self.properties,
        };

        (file_caches, global_cache)
    }

    /// Render the type with struct/enum fields expanded
    fn expanded_ty(&self, ty: &ast::Type) -> String {
        match ty {
            ast::Type::Ref(ty) => self.expanded_ty(ty),
            ast::Type::StorageRef(_, ty) => self.expanded_ty(ty),
            ast::Type::Struct(struct_type) => {
                let strct = struct_type.definition(self.ns);
                let mut tags = render(&strct.tags);
                if !tags.is_empty() {
                    tags.push_str("\n\n")
                }

                let fields = strct
                    .fields
                    .iter()
                    .map(|field| {
                        format!("\t{} {}", field.ty.to_string(self.ns), field.name_as_str())
                    })
                    .join(",\n");

                let val = make_code_block(format!("struct {strct} {{\n{fields}\n}}"));
                format!("{tags}{val}")
            }
            ast::Type::Enum(n) => {
                let enm = &self.ns.enums[*n];
                let mut tags = render(&enm.tags);
                if !tags.is_empty() {
                    tags.push_str("\n\n")
                }
                let values = enm
                    .values
                    .iter()
                    .map(|value| format!("\t{}", value.0))
                    .join(",\n");

                let val = make_code_block(format!("enum {enm} {{\n{values}\n}}"));
                format!("{tags}{val}")
            }
            _ => make_code_block(ty.to_string(self.ns)),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for SolangServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                    completion_item: None,
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: None,
                    retrigger_characters: None,
                    work_done_progress_options: Default::default(),
                }),
                document_highlight_provider: None,
                workspace_symbol_provider: Some(OneOf::Left(true)),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![],
                    work_done_progress_options: Default::default(),
                }),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                definition_provider: Some(OneOf::Left(true)),
                type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
                implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
                declaration_provider: Some(DeclarationCapability::Simple(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "solang language server {} initialized",
                    env!("SOLANG_VERSION")
                ),
            )
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        self.client
            .log_message(MessageType::INFO, "workspace folders changed!")
            .await;
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        self.client
            .log_message(MessageType::INFO, "configuration changed!")
            .await;
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.client
            .log_message(MessageType::INFO, "watched files have changed!")
            .await;
    }

    async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
        self.client
            .log_message(MessageType::INFO, "command executed!")
            .await;
        Ok(None)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;

        match uri.to_file_path() {
            Ok(path) => {
                self.files
                    .lock()
                    .await
                    .text_buffers
                    .insert(path, params.text_document.text);
                self.parse_file(uri).await;
            }
            Err(_) => {
                self.client
                    .log_message(MessageType::ERROR, format!("received invalid URI: {uri}"))
                    .await;
            }
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        match uri.to_file_path() {
            Ok(path) => {
                if let Some(text_buf) = self.files.lock().await.text_buffers.get_mut(&path) {
                    *text_buf = params
                        .content_changes
                        .into_iter()
                        .fold(text_buf.clone(), update_file_contents);
                }
                self.parse_file(uri).await;
            }
            Err(_) => {
                self.client
                    .log_message(MessageType::ERROR, format!("received invalid URI: {uri}"))
                    .await;
            }
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;

        if let Some(text) = params.text {
            if let Ok(path) = uri.to_file_path() {
                if let Some(text_buf) = self.files.lock().await.text_buffers.get_mut(&path) {
                    *text_buf = text;
                }
            }
        }

        self.parse_file(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        if let Ok(path) = uri.to_file_path() {
            let mut files = self.files.lock().await;
            files.caches.remove(&path);
            files.text_buffers.remove(&path);
        }

        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    /// Called when the client raises a `textDocument/completion` request.
    /// There are two kinds of requests that are handled differently:
    /// * Triggered by user pressing `.`
    ///     - In this case, we return a list of fields, variants or methods defined on the code object
    ///       associated with the `.` which triggered the request.
    /// * All other cases where the request is raised by user typing characters other than `.`
    ///     - Here, we return a list of variables, structs, enums, contracts, functions etc. accessible from the current scope.
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let path = uri.to_file_path().map_err(|_| Error {
            code: ErrorCode::InvalidRequest,
            message: format!("Received invalid URI: {uri}").into(),
            data: None,
        })?;

        let files = self.files.lock().await;

        let Some(cache) = files.caches.get(&path) else {
            return Ok(None);
        };

        let offset = cache
            .file
            .get_offset(
                params.text_document_position.position.line as _,
                params.text_document_position.position.character as _,
            )
            .unwrap();

        let builtin_functions = BUILTIN_FUNCTIONS
            .iter()
            .filter(|function| function.target.is_empty() || function.target.contains(&self.target))
            .map(|function| (function.name.to_string(), None));
        let builtin_variables = BUILTIN_VARIABLE
            .iter()
            .filter(|var| var.target.is_empty() || var.target.contains(&self.target))
            .map(|var| (var.name.to_string(), None));

        // Get all the code objects available from the lexical scope from which the request was raised.
        let code_objects_in_scope = cache
            .scopes
            .find(offset, offset + 1)
            // get all the enclosing scopes
            .flat_map(|scope| scope.val.iter().cloned())
            // get the top level code objects in the file
            .chain(cache.top_level_code_objects.clone())
            // builtins
            .chain(builtin_functions)
            .chain(builtin_variables)
            .collect::<HashMap<_, _>>();

        let global_cache = self.global_cache.lock().await;

        let suggestions = match params.context {
            Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some(trigger_character),
            }) if trigger_character == "." => {
                let Some(text_buf) = files.text_buffers.get(&path) else {
                    return Ok(None);
                };

                let mut builtin_methods =
                    HashMap::<DefinitionType, HashMap<String, Option<DefinitionIndex>>>::new();
                for method in BUILTIN_METHODS.iter().filter(|method| {
                    method.target.is_empty() || method.target.contains(&self.target)
                }) {
                    if let Some(def_type) = get_type_definition(&method.method[0]) {
                        builtin_methods
                            .entry(def_type)
                            .or_default()
                            .insert(method.name.to_string(), None);
                    }
                }

                let builtin_structs = BUILTIN_STRUCTS
                    .iter()
                    .map(|r#struct| {
                        let def_type = DefinitionType::Struct(r#struct.struct_type);
                        let fields = r#struct
                            .struct_decl
                            .fields
                            .iter()
                            .map(|field| (field.name_as_str().to_string(), None))
                            .collect();
                        (def_type, fields)
                    })
                    .collect::<HashMap<_, HashMap<_, _>>>();

                // Extract code object from source code for which `Completion` request was triggered.
                // Extracts all the characters connected to the "." character.
                // This includes all the alphanumeric characters that come before the triggering "."
                // and the interspersed "." characters between the alphanumeric characters.
                let code_object = {
                    let buffer = text_buf.chars().collect_vec();
                    let mut curr: isize = offset as isize - 2;
                    while curr >= 0
                        && (buffer[curr as usize].is_ascii_alphanumeric()
                            || buffer[curr as usize] == '.')
                    {
                        curr -= 1;
                    }
                    curr = isize::max(curr, 0);
                    if !buffer[curr as usize].is_ascii_alphanumeric() {
                        curr += 1;
                    }
                    let name = buffer[curr as usize..offset - 1].iter().collect::<String>();

                    name
                };

                // Get an iterator that iterates over all parts of the code object.
                // The parts are basically a field, a variant or a method defined on the previous part.
                let mut code_object_parts = code_object.split('.');

                // `properties` gives the list of fields, variants and methods defined for the code object in question.
                let properties = code_object_parts.next().and_then(|symbol| {
                    code_objects_in_scope
                        .get(symbol)
                        .and_then(|def_index| def_index.as_ref())
                        .and_then(|def_index| {
                            global_cache
                                .properties
                                .get(def_index)
                                .or_else(|| builtin_methods.get(&def_index.def_type))
                                .or_else(|| builtin_structs.get(&def_index.def_type))
                        })
                });
                let properties = code_object_parts.fold(properties, |acc, prop| {
                    acc.and_then(|properties| properties.get(prop))
                        .and_then(|def_index| def_index.as_ref())
                        .and_then(|def_index| {
                            global_cache
                                .properties
                                .get(def_index)
                                .or_else(|| builtin_methods.get(&def_index.def_type))
                                .or_else(|| builtin_structs.get(&def_index.def_type))
                        })
                });

                // Return a list of suggestions using the `properties` extracted previously by converting them into the expected format.
                properties.map(|properties| {
                    properties
                        .keys()
                        .map(|name| CompletionItem {
                            label: name.clone(),
                            ..Default::default()
                        })
                        .collect_vec()
                })
            }
            Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                ..
            }) => {
                let suggestions = code_objects_in_scope
                    .into_keys()
                    .map(|label| CompletionItem {
                        label: label.clone(),
                        ..Default::default()
                    })
                    .collect_vec();
                Some(suggestions)
            }
            _ => None,
        };

        Ok(suggestions.map(CompletionResponse::Array))
    }

    async fn hover(&self, hverparam: HoverParams) -> Result<Option<Hover>> {
        let txtdoc = hverparam.text_document_position_params.text_document;
        let pos = hverparam.text_document_position_params.position;

        let uri = txtdoc.uri;

        if let Ok(path) = uri.to_file_path() {
            let files = &self.files.lock().await;
            if let Some(cache) = files.caches.get(&path) {
                if let Some(offset) = cache
                    .file
                    .get_offset(pos.line as usize, pos.character as usize)
                {
                    // The shortest hover for the position will be most informative
                    if let Some(hover) = cache
                        .hovers
                        .find(offset, offset + 1)
                        .min_by(|a, b| (a.stop - a.start).cmp(&(b.stop - b.start)))
                    {
                        let range = get_range_exclusive(hover.start, hover.stop, &cache.file);

                        return Ok(Some(Hover {
                            contents: HoverContents::Scalar(MarkedString::from_markdown(
                                hover.val.to_string(),
                            )),
                            range: Some(range),
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Called when "Go to Definition" is called by the user on the client side.
    ///
    /// Expected to return the location in source code where the given code object is defined.
    ///
    /// ### Arguments
    /// * `GotoDefinitionParams` provides the source code location (filename, line number, column number) of the code object for which the request was made.
    ///
    /// ### Edge cases
    /// * Returns `Err` when an invalid file path is received.
    /// * Returns `Ok(None)` when the code object is not defined in user's source code. For example, built-in functions.
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        // fetch the `DefinitionIndex` of the code object
        let Some(reference) = self.get_reference_from_params(params).await? else {
            return Ok(None);
        };

        // get the location of the definition of the code object in source code
        let definitions = &self.global_cache.lock().await.definitions;
        let location = definitions
            .get(&reference)
            .map(|range| {
                let uri = Url::from_file_path(&reference.def_path).unwrap();
                Location { uri, range: *range }
            })
            .map(GotoTypeDefinitionResponse::Scalar);

        Ok(location)
    }

    /// Called when "Go to Type Definition" is called by the user on the client side.
    ///
    /// Expected to return the type of the given code object (variable, struct field etc).
    ///
    /// ### Arguments
    /// * `GotoTypeDefinitionParams` provides the source code location (filename, line number, column number) of the code object for which the request was made.
    ///
    /// ### Edge cases
    /// * Returns `Err` when an invalid file path is received.
    /// * Returns `Ok(None)`
    ///     * when the code object is not defined in user's source code. For example, built-in types.
    ///     * if the code object is itself a type.
    async fn goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> Result<Option<GotoTypeDefinitionResponse>> {
        // fetch the `DefinitionIndex` of the code object in question
        let Some(reference) = self.get_reference_from_params(params).await? else {
            return Ok(None);
        };

        let gc = self.global_cache.lock().await;

        // get the `DefinitionIndex` of the type of the given code object
        let di = match &reference.def_type {
            DefinitionType::Variable(_)
            | DefinitionType::NonLocalVariable(_, _)
            | DefinitionType::Field(_, _)
            | DefinitionType::Variant(_, _) => {
                if let Some(def) = gc.types.get(&reference) {
                    def
                } else {
                    return Ok(None);
                }
            }
            // return `Ok(None)` if the code object is itself a type
            DefinitionType::Struct(_)
            | DefinitionType::Enum(_)
            | DefinitionType::Contract(_)
            | DefinitionType::Event(_)
            | DefinitionType::UserType(_) => &reference,
            _ => return Ok(None),
        };

        // get the location of the definition of the type in source code
        let location = gc
            .definitions
            .get(di)
            .map(|range| {
                let uri = Url::from_file_path(&di.def_path).unwrap();
                Location { uri, range: *range }
            })
            .map(GotoTypeDefinitionResponse::Scalar);

        Ok(location)
    }

    /// Called when "Go to Implementations" is called by the user on the client side.
    ///
    /// Expected to return a list (possibly empty) of methods defined for the given contract.
    ///
    /// ### Arguments
    /// * `GotoImplementationParams` provides the source code location (filename, line number, column number) of the code object for which the request was made.
    ///
    /// ### Edge cases
    /// * Returns `Err` when an invalid file path is received.
    /// * Returns `Ok(None)` when the location passed in the arguments doesn't belong to a contract defined in user code.
    async fn goto_implementation(
        &self,
        params: GotoImplementationParams,
    ) -> Result<Option<GotoImplementationResponse>> {
        // fetch the `DefinitionIndex` of the code object in question
        let Some(reference) = self.get_reference_from_params(params).await? else {
            return Ok(None);
        };

        let gc = self.global_cache.lock().await;

        // get the list of `DefinitionIndex` of all the methods defined in the given contract
        // `None` if the passed code-object is not of type `Contract`
        let impls = match &reference.def_type {
            DefinitionType::Variable(_)
            | DefinitionType::NonLocalVariable(_, _)
            | DefinitionType::Field(_, _) => gc.types.get(&reference).and_then(|ty| {
                if matches!(ty.def_type, DefinitionType::Contract(_)) {
                    gc.implementations.get(ty)
                } else {
                    None
                }
            }),
            DefinitionType::Contract(_) => gc.implementations.get(&reference),
            _ => None,
        };

        // get the locations of the definition of methods in source code
        let impls = impls
            .map(|impls| {
                impls
                    .iter()
                    .filter_map(|di| {
                        let path = &di.def_path;
                        gc.definitions.get(di).map(|range| {
                            let uri = Url::from_file_path(path).unwrap();
                            Location { uri, range: *range }
                        })
                    })
                    .collect()
            })
            .map(GotoImplementationResponse::Array);

        Ok(impls)
    }

    /// Called when "Go to Declaration" is called by the user on the client side.
    ///
    /// Expected to return a list (possibly empty) of methods that the given method overrides.
    /// Only the methods belonging to the immediate parent contracts (due to multiple inheritance, there can be more than one parent) are to be returned.
    ///
    /// ### Arguments
    /// * `GotoDeclarationParams` provides the source code location (filename, line number, column number) of the code object for which the request was made.
    ///
    /// ### Edge cases
    /// * Returns `Err` when an invalid file path is received.
    /// * Returns `Ok(None)` when the location passed in the arguments doesn't belong to a contract method defined in user code.
    async fn goto_declaration(
        &self,
        params: GotoDeclarationParams,
    ) -> Result<Option<GotoDeclarationResponse>> {
        // fetch the `DefinitionIndex` of the code object in question
        let Some(reference) = self.get_reference_from_params(params).await? else {
            return Ok(None);
        };

        let gc = self.global_cache.lock().await;

        // get a list of `DefinitionIndex`s of overridden functions from parent contracts
        let decls = gc.declarations.get(&reference);

        // get a list of locations in source code where the overridden functions are present
        let locations = decls
            .map(|decls| {
                decls
                    .iter()
                    .filter_map(|di| {
                        let path = &di.def_path;
                        gc.definitions.get(di).map(|range| {
                            let uri = Url::from_file_path(path).unwrap();
                            Location { uri, range: *range }
                        })
                    })
                    .collect()
            })
            .map(GotoImplementationResponse::Array);

        Ok(locations)
    }

    /// Called when "Go to References" is called by the user on the client side.
    ///
    /// Expected to return a list of locations in the source code where the given code-object is used.
    ///
    /// ### Arguments
    /// * `ReferenceParams`
    ///     * provides the source code location (filename, line number, column number) of the code object for which the request was made.
    ///     * says if the definition location is to be included in the list of source code locations returned or not.
    ///
    /// ### Edge cases
    /// * Returns `Err` when an invalid file path is received.
    /// * Returns `Ok(None)` when no valid references are found.
    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        // fetch the `DefinitionIndex` of the code object in question
        let def_params: GotoDefinitionParams = GotoDefinitionParams {
            text_document_position_params: params.text_document_position,
            work_done_progress_params: params.work_done_progress_params,
            partial_result_params: params.partial_result_params,
        };
        let Some(reference) = self.get_reference_from_params(def_params).await? else {
            return Ok(None);
        };

        // fetch all the locations in source code where the code object is referenced
        // this includes the definition location of the code object
        let caches = &self.files.lock().await.caches;
        let mut locations: Vec<_> = caches
            .iter()
            .flat_map(|(p, cache)| {
                let uri = Url::from_file_path(p).unwrap();
                cache
                    .references
                    .iter()
                    .filter(|r| r.val == reference)
                    .map(move |r| Location {
                        uri: uri.clone(),
                        range: get_range_exclusive(r.start, r.stop, &cache.file),
                    })
            })
            .collect();

        // remove the definition location if `include_declaration` is `false`
        if !params.context.include_declaration {
            let definitions = &self.global_cache.lock().await.definitions;
            let uri = Url::from_file_path(&reference.def_path).unwrap();
            if let Some(range) = definitions.get(&reference) {
                let def = Location { uri, range: *range };
                locations.retain(|loc| loc != &def);
            }
        }

        // return `None` if the list of locations is empty
        let locations = if locations.is_empty() {
            None
        } else {
            Some(locations)
        };

        Ok(locations)
    }

    /// Called when "Rename Symbol" is called by the user on the client side.
    ///
    /// Expected to return a list of changes to be made in user code so that every occurrence of the code object is renamed.
    ///
    /// ### Arguments
    /// * `RenameParams`
    ///     * provides the source code location (filename, line number, column number) of the code object for which the request was made.
    ///     * the new symbol that the code object should go by.
    ///
    /// ### Edge cases
    /// * Returns `Err` when an invalid file path is received.
    /// * Returns `Ok(None)` when the definition of code object is not found in user code.
    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        // fetch the `DefinitionIndex` of the code object in question
        let def_params: GotoDefinitionParams = GotoDefinitionParams {
            text_document_position_params: params.text_document_position,
            work_done_progress_params: params.work_done_progress_params,
            partial_result_params: Default::default(),
        };
        let Some(reference) = self.get_reference_from_params(def_params).await? else {
            return Ok(None);
        };

        // the new name of the code object
        let new_text = params.new_name;

        // create `TextEdit` instances that represent the changes to be made for every occurrence of the old symbol
        // these `TextEdit` objects are then grouped into separate list per source file to which they belong
        let caches = &self.files.lock().await.caches;
        let ws = caches
            .iter()
            .map(|(p, cache)| {
                let uri = Url::from_file_path(p).unwrap();
                let text_edits: Vec<_> = cache
                    .references
                    .iter()
                    .filter(|r| r.val == reference)
                    .map(|r| TextEdit {
                        range: get_range_exclusive(r.start, r.stop, &cache.file),
                        new_text: new_text.clone(),
                    })
                    .collect();
                (uri, text_edits)
            })
            .collect::<HashMap<_, _>>();

        Ok(Some(WorkspaceEdit::new(ws)))
    }

    /// Called when "Format Document" is called by the user on the client side.
    ///
    /// Expected to return the formatted version of source code present in the file on which this method was triggered.
    ///
    /// ### Arguments
    /// * `DocumentFormattingParams`
    ///     * provides the name of the file whose code is to be formatted.
    ///     * provides options that help configure how the file is formatted.
    ///
    /// ### Edge cases
    /// * Returns `Err` when
    ///     * an invalid file path is received.
    ///     * reading the file fails.
    ///     * parsing the file fails.
    ///     * formatting the file fails.
    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        // get parse tree for the input file
        let uri = params.text_document.uri;
        let source_path = uri.to_file_path().map_err(|_| Error {
            code: ErrorCode::InvalidRequest,
            message: format!("Received invalid URI: {uri}").into(),
            data: None,
        })?;
        let source = std::fs::read_to_string(source_path).map_err(|err| Error {
            code: ErrorCode::InternalError,
            message: format!("Failed to read file: {uri}").into(),
            data: Some(Value::String(format!("{:?}", err))),
        })?;
        let source_parsed = parse(&source).map_err(|err| {
            let err = err
                .into_iter()
                .map(|e| Value::String(e.message))
                .collect::<Vec<_>>();
            Error {
                code: ErrorCode::InternalError,
                message: format!("Failed to parse file: {uri}").into(),
                data: Some(Value::Array(err)),
            }
        })?;

        // get the formatted text
        let config = FormatterConfig {
            line_length: 80,
            tab_width: params.options.tab_size as _,
            ..Default::default()
        };
        let mut source_formatted = String::new();
        format_to(&mut source_formatted, source_parsed, config).map_err(|err| Error {
            code: ErrorCode::InternalError,
            message: format!("Failed to format file: {uri}").into(),
            data: Some(Value::String(format!("{:?}", err))),
        })?;

        // create a `TextEdit` instance that replaces the contents of the file with the formatted text
        let text_edit = TextEdit {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: u32::MAX,
                    character: u32::MAX,
                },
            },
            new_text: source_formatted,
        };

        Ok(Some(vec![text_edit]))
    }
}

/// Calculate the line and column from the Loc offset received from the parser
fn loc_to_range(loc: &pt::Loc, file: &ast::File) -> Range {
    get_range(loc.start(), loc.end(), file)
}

fn get_range(start: usize, end: usize, file: &ast::File) -> Range {
    let (line, column) = file.offset_to_line_column(start);
    let start = Position::new(line as u32, column as u32);
    let (line, column) = file.offset_to_line_column(end);
    let end = Position::new(line as u32, column as u32);

    Range::new(start, end)
}

// Get `Range` when the parameters passed represent a half open range of type [start, stop)
// Used when `Range` is to be extracted from `Interval` from the `rust_lapper` crate.
fn get_range_exclusive(start: usize, end: usize, file: &ast::File) -> Range {
    get_range(start, end - 1, file)
}

fn get_type_definition(ty: &Type) -> Option<DefinitionType> {
    match ty {
        Type::Enum(id) => Some(DefinitionType::Enum(*id)),
        Type::Struct(st) => Some(DefinitionType::Struct(*st)),
        Type::Array(ty, _) => get_type_definition(ty),
        Type::Ref(ty) => get_type_definition(ty),
        Type::StorageRef(_, ty) => get_type_definition(ty),
        Type::Contract(id) => Some(DefinitionType::Contract(*id)),
        Type::UserType(id) => Some(DefinitionType::UserType(*id)),
        Type::DynamicBytes => Some(DefinitionType::DynamicBytes),
        _ => None,
    }
}

fn make_code_block(s: impl AsRef<str>) -> String {
    format!("```solidity\n{}\n```", s.as_ref())
}

fn get_constants(expr: &Expression) -> Option<String> {
    let val = match expr {
        codegen::Expression::BytesLiteral {
            ty: ast::Type::Bytes(_) | ast::Type::DynamicBytes,
            value,
            ..
        } => {
            format!("hex\"{}\"", hex::encode(value))
        }
        codegen::Expression::BytesLiteral {
            ty: ast::Type::String,
            value,
            ..
        } => {
            format!("\"{}\"", String::from_utf8_lossy(value))
        }
        codegen::Expression::NumberLiteral {
            ty: ast::Type::Uint(_) | ast::Type::Int(_),
            value,
            ..
        } => {
            format!("{value}")
        }
        _ => return None,
    };
    Some(val)
}

fn update_file_contents(
    mut prev_content: String,
    content_change: TextDocumentContentChangeEvent,
) -> String {
    if let Some(range) = content_change.range {
        let start_line = range.start.line as usize;
        let start_col = range.start.character as usize;
        let end_line = range.end.line as usize;
        let end_col = range.end.character as usize;

        // Directly add the changes to the buffer when changes are present at the end of the file.
        if start_line == prev_content.lines().count() {
            prev_content.push_str(&content_change.text);
            return prev_content;
        }

        let mut new_content = String::new();
        for (i, line) in prev_content.lines().enumerate() {
            if i < start_line {
                new_content.push_str(line);
                new_content.push('\n');
                continue;
            }

            if i > end_line {
                new_content.push_str(line);
                new_content.push('\n');
                continue;
            }

            if i == start_line {
                new_content.push_str(&line[..start_col]);
                new_content.push_str(&content_change.text);
            }

            if i == end_line {
                new_content.push_str(&line[end_col..]);
                new_content.push('\n');
            }
        }
        new_content
    } else {
        // When no range is provided, entire file is sent in the request.
        content_change.text
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn without_range() {
        let initial_content = "contract foo {\n    function bar(Book y, Book x) public returns (bool) {\n        return y.available;\n    }\n}\n".to_string();
        let new_content = "struct Book {\n    string name;\n    string writer;\n    uint id;\n    bool available;\n}\n".to_string();
        assert_eq!(
            new_content.clone(),
            update_file_contents(
                initial_content,
                TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: new_content
                }
            )
        );
    }

    #[test]
    fn at_the_end_of_file() {
        let initial_content = "contract foo {\n    function bar(Book y, Book x) public returns (bool) {\n        return y.available;\n    }\n}\n".to_string();
        let new_content = "struct Book {\n    string name;\n    string writer;\n    uint id;\n    bool available;\n}\n".to_string();
        let final_content = "\
            contract foo {\n    function bar(Book y, Book x) public returns (bool) {\n        return y.available;\n    }\n}\n\
            struct Book {\n    string name;\n    string writer;\n    uint id;\n    bool available;\n}\n\
        ".to_string();
        assert_eq!(
            final_content,
            update_file_contents(
                initial_content,
                TextDocumentContentChangeEvent {
                    range: Some(Range {
                        start: Position {
                            line: 5,
                            character: 0
                        },
                        end: Position {
                            line: 5,
                            character: 0
                        }
                    }),
                    range_length: Some(0),
                    text: new_content
                }
            ),
        );
    }

    #[test]
    fn remove_content() {
        let initial_content = "struct Book {\n    string name;\n    string writer;\n    uint id;\n    bool available;\n}\n".to_string();
        let final_content =
            "struct Book {\n    string name;\n    string id;\n    bool available;\n}\n".to_string();
        assert_eq!(
            final_content,
            update_file_contents(
                initial_content,
                TextDocumentContentChangeEvent {
                    range: Some(Range {
                        start: Position {
                            line: 2,
                            character: 11
                        },
                        end: Position {
                            line: 3,
                            character: 9
                        }
                    }),
                    range_length: Some(17),
                    text: String::new(),
                }
            ),
        );
    }

    #[test]
    fn add_content() {
        let initial_content =
            "struct Book {\n    string name;\n    string id;\n    bool available;\n}\n".to_string();
        let final_content = "struct Book {\n    string name;\n    string writer;\n    uint id;\n    bool available;\n}\n".to_string();
        assert_eq!(
            final_content,
            update_file_contents(
                initial_content,
                TextDocumentContentChangeEvent {
                    range: Some(Range {
                        start: Position {
                            line: 2,
                            character: 11
                        },
                        end: Position {
                            line: 2,
                            character: 11
                        }
                    }),
                    range_length: Some(0),
                    text: "writer;\n    uint ".to_string(),
                }
            ),
        );
    }
}
