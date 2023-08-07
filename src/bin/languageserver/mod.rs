// SPDX-License-Identifier: Apache-2.0

use itertools::Itertools;
use rust_lapper::{Interval, Lapper};
use serde_json::Value;
use solang::{
    codegen::codegen,
    codegen::{self, Expression},
    file_resolver::FileResolver,
    parse_and_resolve,
    sema::{ast, builtin::get_prototype, symtable, tags::render},
    Target,
};
use solang_parser::pt;
use std::{collections::HashMap, ffi::OsString, path::PathBuf};
use tokio::sync::Mutex;
use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{
        CompletionOptions, CompletionParams, CompletionResponse, Diagnostic,
        DiagnosticRelatedInformation, DiagnosticSeverity, DidChangeConfigurationParams,
        DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
        ExecuteCommandOptions, ExecuteCommandParams, Hover, HoverContents, HoverParams,
        HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams, Location,
        MarkedString, MessageType, OneOf, Position, Range, ServerCapabilities,
        SignatureHelpOptions, TextDocumentContentChangeEvent, TextDocumentSyncCapability,
        TextDocumentSyncKind, Url, WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
    },
    Client, LanguageServer, LspService, Server,
};

use crate::cli::{target_arg, LanguageServerCommand};

struct Hovers {
    file: ast::File,
    hovers: Lapper<usize, String>,
}

type HoverEntry = Interval<usize, String>;

/// Stores information used by language server for every opened file
struct Files {
    hovers: HashMap<PathBuf, Hovers>,
    text_buffers: HashMap<PathBuf, String>,
}

pub struct SolangServer {
    client: Client,
    target: Target,
    importpaths: Vec<PathBuf>,
    importmaps: Vec<(String, PathBuf)>,
    files: Mutex<Files>,
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
        files: Mutex::new(Files {
            hovers: HashMap::new(),
            text_buffers: HashMap::new(),
        }),
        importpaths,
        importmaps,
    });

    Server::new(stdin, stdout, socket).serve(service).await;

    std::process::exit(1);
}

impl SolangServer {
    /// Parse file
    async fn parse_file(&self, uri: Url) {
        let mut resolver = FileResolver::new();
        for (path, contents) in &self.files.lock().await.text_buffers {
            resolver.set_file_contents(path.to_str().unwrap(), contents.clone());
        }
        if let Ok(path) = uri.to_file_path() {
            let dir = path.parent().unwrap();

            let _ = resolver.add_import_path(dir);

            let mut diags = Vec::new();

            for path in &self.importpaths {
                if let Err(e) = resolver.add_import_path(path) {
                    diags.push(Diagnostic {
                        message: format!("import path '{}': {}", path.to_string_lossy(), e),
                        severity: Some(DiagnosticSeverity::ERROR),
                        ..Default::default()
                    });
                }
            }

            for (map, path) in &self.importmaps {
                if let Err(e) = resolver.add_import_map(OsString::from(map), PathBuf::from(path)) {
                    diags.push(Diagnostic {
                        message: format!("error: import path '{}': {e}", path.display()),
                        severity: Some(DiagnosticSeverity::ERROR),
                        ..Default::default()
                    });
                }
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

            let hovers = Builder::build(&ns);

            self.files.lock().await.hovers.insert(path, hovers);

            res.await;
        }
    }
}

struct Builder<'a> {
    hovers: Vec<HoverEntry>,
    ns: &'a ast::Namespace,
}

impl<'a> Builder<'a> {
    // Constructs lookup table(messages) for the given statement by traversing the
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

                self.hovers.push(HoverEntry {
                    start: param.loc.start(),
                    stop: param.loc.exclusive_end(),
                    val: make_code_block(val),
                });
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
                        ast::DestructureField::VariableDecl(_, param) => {
                            self.hovers.push(HoverEntry {
                                start: param.loc.start(),
                                stop: param.loc.exclusive_end(),
                                val: self.expanded_ty(&param.ty),
                            });
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
                self.hovers.push(HoverEntry {
                    start: event_loc.start(),
                    stop: event_loc.exclusive_end(),
                    val: format!("{}{}", tags, make_code_block(val)),
                });

                for arg in args {
                    self.expression(arg, symtab);
                }
            }
            ast::Statement::TryCatch(_, _, try_stmt) => {
                self.expression(&try_stmt.expr, symtab);
                for stmt in &try_stmt.catch_stmt {
                    self.statement(stmt, symtab);
                }
                for stmt in &try_stmt.ok_stmt {
                    self.statement(stmt, symtab);
                }
                for (_, _, block) in &try_stmt.errors {
                    for stmts in block {
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

    // Constructs lookup table(messages) by traversing over the expressions and storing
    // the respective expression type messages in the table.
    fn expression(&mut self, expr: &ast::Expression, symtab: &symtable::Symtable) {
        match expr {
            // Variable types expression
            ast::Expression::BoolLiteral { loc, .. } => {
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: make_code_block("bool"),
                });
            }
            ast::Expression::BytesLiteral { loc, ty, .. } => {
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: self.expanded_ty(ty),
                });
            }
            ast::Expression::CodeLiteral { loc, .. } => {
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: make_code_block("bytes"),
                });
            }
            ast::Expression::NumberLiteral { loc, ty, .. } => {
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: make_code_block(ty.to_string(self.ns)),
                });
            }
            ast::Expression::StructLiteral { values, .. }
            | ast::Expression::ArrayLiteral { values, .. }
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
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: format!(
                        "{} {} addition",
                        if *unchecked { "unchecked " } else { "" },
                        ty.to_string(self.ns)
                    ),
                });

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
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: format!(
                        "{} {} subtraction",
                        if *unchecked { "unchecked " } else { "" },
                        ty.to_string(self.ns)
                    ),
                });

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
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: format!(
                        "{} {} multiply",
                        if *unchecked { "unchecked " } else { "" },
                        ty.to_string(self.ns)
                    ),
                });

                self.expression(left, symtab);
                self.expression(right, symtab);
            }
            ast::Expression::Divide {
                loc,
                ty,
                left,
                right,
            } => {
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: format!("{} divide", ty.to_string(self.ns)),
                });

                self.expression(left, symtab);
                self.expression(right, symtab);
            }
            ast::Expression::Modulo {
                loc,
                ty,
                left,
                right,
            } => {
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: format!("{} modulo", ty.to_string(self.ns)),
                });

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
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: format!(
                        "{} {}power",
                        if *unchecked { "unchecked " } else { "" },
                        ty.to_string(self.ns)
                    ),
                });

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

                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: make_code_block(val),
                });
            }
            ast::Expression::ConstantVariable { loc, ty, contract_no, var_no } => {
                let (contract, name) = if let Some(contract_no) = contract_no {
                    let contract = format!("{}.", self.ns.contracts[*contract_no].name);
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
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: make_code_block(val),
                });
            }
            ast::Expression::StorageVariable { loc, ty, contract_no, var_no } => {
                let contract = &self.ns.contracts[*contract_no];
                let name = &contract.variables[*var_no].name;
                let val = format!("{} {}.{}", ty.to_string(self.ns), contract.name, name);
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: make_code_block(val),
                });
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

            ast::Expression::StructMember { loc, ty, expr, .. } => {
                self.expression(expr, symtab);
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: make_code_block(ty.to_string(self.ns)),
                });
            }

            // Array operation expression
            ast::Expression::AllocDynamicBytes {  length,  .. } => {
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
            ast::Expression::StringConcat { left, right, .. } => {
                if let ast::StringLocation::RunTime(expr) = left {
                    self.expression(expr, symtab);
                }
                if let ast::StringLocation::RunTime(expr) = right {
                    self.expression(expr, symtab);
                }
            }

            // Function call expression
            ast::Expression::InternalFunctionCall {
                loc,
                function,
                args,
                ..
            } => {
                if let ast::Expression::InternalFunction { function_no, .. } = function.as_ref() {
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

                    let contract = fnc.contract_no.map(|contract_no| format!("{}.", self.ns.contracts[contract_no].name)).unwrap_or_default();

                    let val = format!("{} {}{}({}) returns ({})\n", fnc.ty, contract, fnc.name, params, rets);

                    self.hovers.push(HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: format!("{}{}", msg_tg, make_code_block(val)),
                    });
                }

                for arg in args {
                    self.expression(arg, symtab);
                }
            }
            ast::Expression::ExternalFunctionCall {
                loc,
                function,
                args,
                call_args,
                ..
            } => {
                if let ast::Expression::ExternalFunction {
                    function_no,
                    address,
                    ..
                } = function.as_ref()
                {
                    // modifiers do not have mutability, bases or modifiers itself
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

                    let contract = fnc.contract_no.map(|contract_no| format!("{}.", self.ns.contracts[contract_no].name)).unwrap_or_default();

                    let val = format!("{} {}{}({}) returns ({})\n", fnc.ty, contract, fnc.name, params, rets);

                    self.hovers.push(HoverEntry {
                        start: loc.start(),
                        stop: loc.exclusive_end(),
                        val: format!("{}{}", msg_tg, make_code_block(val)),
                    });

                    self.expression(address, symtab);
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
                if let Some(address) = &call_args.address {
                    self.expression(address, symtab);
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
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: format!("{doc}{val}"),
                });

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
    fn contract_variable(&mut self, contract: &ast::Variable, symtab: &symtable::Symtable) {
        let mut tags = render(&contract.tags[..]);
        if !tags.is_empty() {
            tags.push_str("\n\n");
        }
        let val = make_code_block(format!(
            "{} {}",
            contract.ty.to_string(self.ns),
            contract.name
        ));
        self.hovers.push(HoverEntry {
            start: contract.loc.start(),
            stop: contract.loc.start() + contract.name.len(),
            val: format!("{tags}{val}"),
        });
        if let Some(expr) = &contract.initializer {
            self.expression(expr, symtab);
        }
    }

    // Constructs struct fields and stores it in the lookup table.
    fn field(&mut self, field: &ast::Parameter) {
        self.hovers.push(HoverEntry {
            start: field.loc.start(),
            stop: field.loc.exclusive_end(),
            val: make_code_block(format!(
                "{} {}",
                field.ty.to_string(self.ns),
                field.name_as_str()
            )),
        });
    }

    // Traverses namespace to build messages stored in the lookup table for hover feature.
    fn build(ns: &ast::Namespace) -> Hovers {
        let mut builder = Builder {
            hovers: Vec::new(),
            ns,
        };

        for enum_decl in &builder.ns.enums {
            for (discriminant, (nam, loc)) in enum_decl.values.iter().enumerate() {
                builder.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.exclusive_end(),
                    val: make_code_block(format!(
                        "enum {}.{} {}",
                        enum_decl.name, nam, discriminant
                    )),
                });
            }

            builder.hovers.push(HoverEntry {
                start: enum_decl.loc.start(),
                stop: enum_decl.loc.start() + enum_decl.name.len(),
                val: render(&enum_decl.tags[..]),
            });
        }

        for struct_decl in &builder.ns.structs {
            if let pt::Loc::File(_, start, _) = &struct_decl.loc {
                for field in &struct_decl.fields {
                    builder.field(field);
                }

                builder.hovers.push(HoverEntry {
                    start: *start,
                    stop: start + struct_decl.name.len(),
                    val: render(&struct_decl.tags[..]),
                });
            }
        }

        for func in &builder.ns.functions {
            if func.is_accessor || func.loc == pt::Loc::Builtin {
                // accessor functions are synthetic; ignore them, all the locations are fake
                continue;
            }

            for note in &func.annotations {
                match note {
                    ast::ConstructorAnnotation::Bump(expr)
                    | ast::ConstructorAnnotation::Seed(expr)
                    | ast::ConstructorAnnotation::Space(expr) => {
                        builder.expression(expr, &func.symtable)
                    }

                    ast::ConstructorAnnotation::Payer(loc, name) => {
                        builder.hovers.push(HoverEntry {
                            start: loc.start(),
                            stop: loc.exclusive_end(),
                            val: format!("payer account: {name}"),
                        });
                    }
                }
            }

            for param in &*func.params {
                builder.hovers.push(HoverEntry {
                    start: param.loc.start(),
                    stop: param.loc.exclusive_end(),
                    val: builder.expanded_ty(&param.ty),
                });
            }

            for ret in &*func.returns {
                builder.hovers.push(HoverEntry {
                    start: ret.loc.start(),
                    stop: ret.loc.exclusive_end(),
                    val: builder.expanded_ty(&ret.ty),
                });
            }

            for stmt in &func.body {
                builder.statement(stmt, &func.symtable);
            }
        }

        for constant in &builder.ns.constants {
            let samptb = symtable::Symtable::new();
            builder.contract_variable(constant, &samptb);
        }

        for contract in &builder.ns.contracts {
            builder.hovers.push(HoverEntry {
                start: contract.loc.start(),
                stop: contract.loc.start() + contract.name.len(),
                val: render(&contract.tags[..]),
            });

            for base in &contract.bases {
                builder.hovers.push(HoverEntry {
                    start: base.loc.start(),
                    stop: base.loc.exclusive_end(),
                    val: make_code_block(format!(
                        "contract {}",
                        builder.ns.contracts[base.contract_no].name
                    )),
                });
            }

            for variable in &contract.variables {
                let symtable = symtable::Symtable::new();
                builder.contract_variable(variable, &symtable);
            }
        }

        for event in &builder.ns.events {
            for field in &event.fields {
                builder.field(field);
            }
            builder.hovers.push(HoverEntry {
                start: event.loc.start(),
                stop: event.loc.start() + event.name.len(),
                val: render(&event.tags[..]),
            });
        }

        for lookup in &mut builder.hovers {
            if let Some(msg) =
                builder
                    .ns
                    .hover_overrides
                    .get(&pt::Loc::File(0, lookup.start, lookup.stop))
            {
                lookup.val = msg.clone();
            }
        }

        Hovers {
            file: ns.files[ns.top_file_no()].clone(),
            hovers: Lapper::new(builder.hovers),
        }
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
            self.files.lock().await.hovers.remove(&path);
            self.files.lock().await.text_buffers.remove(&path);
        }

        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(None)
    }

    async fn hover(&self, hverparam: HoverParams) -> Result<Option<Hover>> {
        let txtdoc = hverparam.text_document_position_params.text_document;
        let pos = hverparam.text_document_position_params.position;

        let uri = txtdoc.uri;

        if let Ok(path) = uri.to_file_path() {
            let files = &self.files.lock().await;
            if let Some(hovers) = files.hovers.get(&path) {
                let offset = hovers
                    .file
                    .get_offset(pos.line as usize, pos.character as usize);

                // The shortest hover for the position will be most informative
                if let Some(hover) = hovers
                    .hovers
                    .find(offset, offset + 1)
                    .min_by(|a, b| (a.stop - a.start).cmp(&(b.stop - b.start)))
                {
                    let loc = pt::Loc::File(0, hover.start, hover.stop);
                    let range = loc_to_range(&loc, &hovers.file);

                    return Ok(Some(Hover {
                        contents: HoverContents::Scalar(MarkedString::from_markdown(
                            hover.val.to_string(),
                        )),
                        range: Some(range),
                    }));
                }
            }
        }

        Ok(None)
    }
}

/// Calculate the line and column from the Loc offset received from the parser
fn loc_to_range(loc: &pt::Loc, file: &ast::File) -> Range {
    let (line, column) = file.offset_to_line_column(loc.start());
    let start = Position::new(line as u32, column as u32);
    let (line, column) = file.offset_to_line_column(loc.end());
    let end = Position::new(line as u32, column as u32);

    Range::new(start, end)
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
