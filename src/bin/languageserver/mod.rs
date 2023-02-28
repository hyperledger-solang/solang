// SPDX-License-Identifier: Apache-2.0

use clap::ArgMatches;
use rust_lapper::{Interval, Lapper};
use serde_json::Value;
use solang::{
    codegen,
    codegen::codegen,
    file_resolver::FileResolver,
    parse_and_resolve,
    sema::{ast, builtin::get_prototype, symtable, tags::render},
    Target,
};
use solang_parser::pt;
use std::{collections::HashMap, ffi::OsString, fmt::Write, path::PathBuf};
use tokio::sync::Mutex;
use tower_lsp::{jsonrpc::Result, lsp_types::*, Client, LanguageServer, LspService, Server};

struct Hovers {
    file: ast::File,
    hovers: Lapper<usize, String>,
}

type HoverEntry = Interval<usize, String>;

pub struct SolangServer {
    client: Client,
    target: Target,
    importpaths: Vec<PathBuf>,
    importmaps: Vec<(String, PathBuf)>,
    files: Mutex<HashMap<PathBuf, Hovers>>,
}

#[tokio::main(flavor = "current_thread")]
pub async fn start_server(target: Target, matches: &ArgMatches) -> ! {
    let mut importpaths = Vec::new();
    let mut importmaps = Vec::new();

    if let Some(paths) = matches.get_many::<PathBuf>("IMPORTPATH") {
        for path in paths {
            importpaths.push(path.to_path_buf());
        }
    }

    if let Some(maps) = matches.get_many::<(String, PathBuf)>("IMPORTMAP") {
        for (map, path) in maps {
            importmaps.push((map.clone(), path.clone()));
        }
    }

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| SolangServer {
        client,
        target,
        files: Mutex::new(HashMap::new()),
        importpaths,
        importmaps,
    });

    Server::new(stdin, stdout, socket).serve(service).await;

    std::process::exit(1);
}

impl SolangServer {
    /// Parse file
    async fn parse_file(&self, uri: Url) {
        if let Ok(path) = uri.to_file_path() {
            let mut resolver = FileResolver::new();

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

            self.files.lock().await.insert(path, hovers);

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
                let mut val = format!("{} {}", self.expanded_ty(&param.ty), param.name_as_str());
                if let Some(expr) = self.ns.var_constants.get(loc) {
                    match expr {
                        codegen::Expression::BytesLiteral(_, ast::Type::Bytes(_), bs)
                        | codegen::Expression::BytesLiteral(_, ast::Type::DynamicBytes, bs) => {
                            write!(val, " = hex\"{}\"", hex::encode(bs)).unwrap();
                        }
                        codegen::Expression::BytesLiteral(_, ast::Type::String, bs) => {
                            write!(val, " = \"{}\"", String::from_utf8_lossy(bs)).unwrap();
                        }
                        codegen::Expression::NumberLiteral(_, ast::Type::Uint(_), n)
                        | codegen::Expression::NumberLiteral(_, ast::Type::Int(_), n) => {
                            write!(val, " = {n}").unwrap();
                        }
                        _ => (),
                    }
                }

                if let Some(var) = symtab.vars.get(var_no) {
                    if var.slice {
                        val.push_str("\nreadonly: compiled to slice\n")
                    }
                }

                self.hovers.push(HoverEntry {
                    start: param.loc.start(),
                    stop: param.loc.end(),
                    val,
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
                for stat in next {
                    self.statement(stat, symtab);
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
                            let val = self.expanded_ty(&param.ty);

                            self.hovers.push(HoverEntry {
                                start: param.loc.start(),
                                stop: param.loc.end(),
                                val,
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
            ast::Statement::Emit {
                event_no,
                event_loc,
                args,
                ..
            } => {
                let event = &self.ns.events[*event_no];

                let mut val = render(&event.tags);

                write!(val, "```\nevent {} {{\n", event.symbol_name(self.ns)).unwrap();

                let mut iter = event.fields.iter().peekable();
                while let Some(field) = iter.next() {
                    writeln!(
                        val,
                        "\t{}{}{}{}",
                        field.ty.to_string(self.ns),
                        if field.indexed { " indexed " } else { " " },
                        field.name_as_str(),
                        if iter.peek().is_some() { "," } else { "" }
                    )
                    .unwrap();
                }

                write!(
                    val,
                    "}}{};\n```\n",
                    if event.anonymous { " anonymous" } else { "" }
                )
                .unwrap();

                self.hovers.push(HoverEntry {
                    start: event_loc.start(),
                    stop: event_loc.end(),
                    val,
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
                    stop: loc.end(),
                    val: "bool".into(),
                });
            }
            ast::Expression::BytesLiteral { loc, ty, .. } => {
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.end(),
                    val: self.expanded_ty(ty),
                });
            }
            ast::Expression::CodeLiteral { loc, .. } => {
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.end(),
                    val: "bytes".into(),
                });
            }
            ast::Expression::NumberLiteral { loc, ty, .. } => {
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.end(),
                    val: ty.to_string(self.ns),
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
                    stop: loc.end(),
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
                    stop: loc.end(),
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
                    stop: loc.end(),
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
                    stop: loc.end(),
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
                    stop: loc.end(),
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
                    stop: loc.end(),
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
                let mut val = self.expanded_ty(ty);

                if let Some(expr) = self.ns.var_constants.get(loc) {
                    match expr {
                        codegen::Expression::BytesLiteral(_, ast::Type::Bytes(_), bs)
                        | codegen::Expression::BytesLiteral(_, ast::Type::DynamicBytes, bs) => {
                            write!(val, " hex\"{}\"", hex::encode(bs)).unwrap();
                        }
                        codegen::Expression::BytesLiteral(_, ast::Type::String, bs) => {
                            write!(val, " \"{}\"", String::from_utf8_lossy(bs)).unwrap();
                        }
                        codegen::Expression::NumberLiteral(_, ast::Type::Uint(_), n)
                        | codegen::Expression::NumberLiteral(_, ast::Type::Int(_), n) => {
                            write!(val, " {n}").unwrap();
                        }
                        _ => (),
                    }
                }

                if let Some(var) = symtab.vars.get(var_no) {
                    if var.slice {
                        val.push_str("\nreadonly: compiles to slice\n")
                    }
                }

                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.end(),
                    val,
                });
            }
            ast::Expression::ConstantVariable { loc, ty, .. } => {
                let val = format!("constant ({})", self.expanded_ty(ty));
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.end(),
                    val,
                });
            }
            ast::Expression::StorageVariable { loc, ty, .. } => {
                let val = format!("({})", self.expanded_ty(ty));
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.end(),
                    val,
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
            | ast::Expression::Complement { expr, .. }
            | ast::Expression::UnaryMinus { expr, .. } => {
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

            ast::Expression::StructMember {  expr, .. } => {
                self.expression(expr, symtab);
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
                    let msg_tg = render(&fnc.tags[..]);

                    let mut val = format!("{} \n\n {} {}(", msg_tg, fnc.ty, fnc.name);

                    for parm in &*fnc.params {
                        let msg = format!(
                            "{}:{}, \n\n",
                            parm.name_as_str(),
                            self.expanded_ty(&parm.ty)
                        );
                        val = format!("{val} {msg}");
                    }

                    val = format!("{val} ) returns (");

                    for ret in &*fnc.returns {
                        let msg = format!(
                            "{}:{}, ",
                            ret.name_as_str(),
                            self.expanded_ty(&ret.ty)
                        );
                        val = format!("{val} {msg}");
                    }

                    val = format!("{val})");
                    self.hovers.push(HoverEntry {
                        start: loc.start(),
                        stop: loc.end(),
                        val,
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
                    let msg_tg = render(&fnc.tags[..]);
                    let mut val = format!("{} \n\n {} {}(", msg_tg, fnc.ty, fnc.name);

                    for parm in &*fnc.params {
                        let msg = format!(
                            "{}:{}, \n\n",
                            parm.name_as_str(),
                            self.expanded_ty(&parm.ty)
                        );
                        val = format!("{val} {msg}");
                    }

                    val = format!("{val} ) \n\n returns (");

                    for ret in &*fnc.returns {
                        let msg = format!(
                            "{}:{}, ",
                            ret.name_as_str(),
                            self.expanded_ty(&ret.ty)
                        );
                        val = format!("{val} {msg}");
                    }

                    val = format!("{val})");
                    self.hovers.push(HoverEntry {
                        start: loc.start(),
                        stop: loc.end(),
                        val,
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
                let mut msg = "[built-in] ".to_string();
                let prot = get_prototype(*kind);

                if let Some(protval) = prot {
                    for ret in &protval.ret {
                        msg = format!("{} {}", msg, self.expanded_ty(ret));
                    }
                    msg = format!("{} {} (", msg, protval.name);
                    for arg in &protval.params {
                        msg = format!("{}{}", msg, self.expanded_ty(arg));
                    }
                    msg = format!("{}): {}", msg, protval.doc);
                }
                self.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.end(),
                    val: msg,
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
        let val = format!("{} {}", self.expanded_ty(&contract.ty), contract.name);
        self.hovers.push(HoverEntry {
            start: contract.loc.start(),
            stop: contract.loc.end(),
            val,
        });
        if let Some(expr) = &contract.initializer {
            self.expression(expr, symtab);
        }
    }

    // Constructs struct fields and stores it in the lookup table.
    fn field(&mut self, field: &ast::Parameter) {
        let val = format!("{} {}", field.ty.to_string(self.ns), field.name_as_str());
        self.hovers.push(HoverEntry {
            start: field.loc.start(),
            stop: field.loc.end(),
            val,
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
                let val = format!("{nam} {discriminant}, \n\n");
                builder.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.end(),
                    val,
                });
            }

            let val = render(&enum_decl.tags[..]);
            builder.hovers.push(HoverEntry {
                start: enum_decl.loc.start(),
                stop: enum_decl.loc.start() + enum_decl.name.len(),
                val,
            });
        }

        for struct_decl in &builder.ns.structs {
            if let pt::Loc::File(_, start, _) = &struct_decl.loc {
                for field in &struct_decl.fields {
                    builder.field(field);
                }

                let val = render(&struct_decl.tags[..]);
                builder.hovers.push(HoverEntry {
                    start: *start,
                    stop: start + struct_decl.name.len(),
                    val,
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
                    | ast::ConstructorAnnotation::Payer(expr)
                    | ast::ConstructorAnnotation::Space(expr) => {
                        builder.expression(expr, &func.symtable)
                    }
                }
            }

            for param in &*func.params {
                let val = builder.expanded_ty(&param.ty);
                builder.hovers.push(HoverEntry {
                    start: param.loc.start(),
                    stop: param.loc.end(),
                    val,
                });
            }

            for ret in &*func.returns {
                let val = builder.expanded_ty(&ret.ty);
                builder.hovers.push(HoverEntry {
                    start: ret.loc.start(),
                    stop: ret.loc.end(),
                    val,
                });
            }

            for stmt in &func.body {
                builder.statement(stmt, &func.symtable);
            }
        }

        for constant in &builder.ns.constants {
            let samptb = symtable::Symtable::new();
            builder.contract_variable(constant, &samptb);

            let val = render(&constant.tags[..]);
            builder.hovers.push(HoverEntry {
                start: constant.loc.start(),
                stop: constant.loc.start() + constant.name.len(),
                val,
            });
        }

        for contract in &builder.ns.contracts {
            let val = render(&contract.tags[..]);
            builder.hovers.push(HoverEntry {
                start: contract.loc.start(),
                stop: contract.loc.start() + val.len(),
                val,
            });

            for variable in &contract.variables {
                let symtable = symtable::Symtable::new();
                builder.contract_variable(variable, &symtable);

                let val = render(&variable.tags[..]);
                builder.hovers.push(HoverEntry {
                    start: variable.loc.start(),
                    stop: variable.loc.start() + variable.name.len(),
                    val,
                });
            }
        }

        for event in &builder.ns.events {
            for field in &event.fields {
                builder.field(field);
            }
            let val = render(&event.tags[..]);
            builder.hovers.push(HoverEntry {
                start: event.loc.start(),
                stop: event.loc.start() + event.name.len(),
                val,
            });
        }

        for lookup in builder.hovers.iter_mut() {
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

                let mut msg = render(&strct.tags);

                writeln!(msg, "```\nstruct {strct} {{").unwrap();

                let mut iter = strct.fields.iter().peekable();
                while let Some(field) = iter.next() {
                    writeln!(
                        msg,
                        "\t{} {}{}",
                        field.ty.to_string(self.ns),
                        field.name_as_str(),
                        if iter.peek().is_some() { "," } else { "" }
                    )
                    .unwrap();
                }

                msg.push_str("};\n```\n");

                msg
            }
            ast::Type::Enum(n) => {
                let enm = &self.ns.enums[*n];

                let mut msg = render(&enm.tags);

                write!(msg, "```\nenum {enm} {{\n").unwrap();

                // display the enum values in-order
                let mut values = Vec::new();
                values.resize(enm.values.len(), "");

                for (idx, value) in enm.values.iter().enumerate() {
                    values[idx] = value.0;
                }

                let mut iter = values.iter().peekable();

                while let Some(value) = iter.next() {
                    writeln!(
                        msg,
                        "\t{}{}",
                        value,
                        if iter.peek().is_some() { "," } else { "" }
                    )
                    .unwrap();
                }

                msg.push_str("};\n```\n");

                msg
            }
            _ => ty.to_string(self.ns),
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
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: None,
                    retrigger_characters: None,
                    work_done_progress_options: Default::default(),
                }),
                document_highlight_provider: None,
                workspace_symbol_provider: Some(OneOf::Left(true)),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["dummy.do_something".to_string()],
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

        self.parse_file(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        self.parse_file(uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;

        self.parse_file(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        if let Ok(path) = uri.to_file_path() {
            self.files.lock().await.remove(&path);
        }
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(None)
    }

    async fn hover(&self, hverparam: HoverParams) -> Result<Option<Hover>> {
        let txtdoc = hverparam.text_document_position_params.text_document;
        let pos = hverparam.text_document_position_params.position;

        let uri = txtdoc.uri;

        if let Ok(path) = uri.to_file_path() {
            let files = self.files.lock().await;
            if let Some(hovers) = files.get(&path) {
                let offset = hovers
                    .file
                    .get_offset(pos.line as usize, pos.character as usize);

                // The shortest hover for the position will be most informative
                if let Some(hover) = hovers
                    .hovers
                    .find(offset, offset)
                    .min_by(|a, b| (a.stop - a.start).cmp(&(b.stop - b.start)))
                {
                    let loc = pt::Loc::File(0, hover.start, hover.stop);
                    let range = loc_to_range(&loc, &hovers.file);

                    return Ok(Some(Hover {
                        contents: HoverContents::Scalar(MarkedString::String(
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
