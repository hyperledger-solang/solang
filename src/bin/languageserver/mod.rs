// SPDX-License-Identifier: Apache-2.0

use clap::ArgMatches;
use num_traits::ToPrimitive;
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
    lookup: Lapper<usize, String>,
}

struct Definitions {
    file: ast::File,
    files: Vec<ast::File>,
    lookup: Lapper<usize, pt::Loc>,
}

type HoverEntry = Interval<usize, String>;
type DefinitionEntry = Interval<usize, pt::Loc>;

struct Intelligence {
    hovers: Vec<HoverEntry>,
    definitions: Vec<DefinitionEntry>,
}

pub struct SolangServer {
    client: Client,
    target: Target,
    importpaths: Vec<PathBuf>,
    importmaps: Vec<String>,
    files: Mutex<HashMap<PathBuf, Hovers>>,
    definitions: Mutex<HashMap<PathBuf, Definitions>>,
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

    if let Some(maps) = matches.get_many::<String>("IMPORTMAP") {
        for map in maps {
            importmaps.push(map.to_string());
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
        definitions: Mutex::new(HashMap::new()),
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

            for p in &self.importmaps {
                if let Some((map, path)) = p.split_once('=') {
                    if let Err(e) =
                        resolver.add_import_map(OsString::from(map), PathBuf::from(path))
                    {
                        diags.push(Diagnostic {
                            message: format!("error: import path '{}': {}", path, e),
                            severity: Some(DiagnosticSeverity::ERROR),
                            ..Default::default()
                        });
                    }
                } else {
                    diags.push(Diagnostic {
                        message: format!("error: import map '{}': contains no '='", p),
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
                                    range: SolangServer::loc_to_range(
                                        &note.loc,
                                        &ns.files[ns.top_file_no()],
                                    ),
                                },
                            })
                            .collect(),
                    )
                };

                let range = SolangServer::loc_to_range(&diag.loc, &ns.files[ns.top_file_no()]);

                Some(Diagnostic {
                    range,
                    message: diag.message.to_string(),
                    severity,
                    related_information,
                    ..Default::default()
                })
            }));

            let res = self.client.publish_diagnostics(uri, diags, None);

            let mut intelligence = Intelligence {
                hovers: Vec::new(),
                definitions: Vec::new(),
            };

            SolangServer::traverse(&ns, &mut intelligence);

            self.files.lock().await.insert(
                path.clone(),
                Hovers {
                    file: ns.files[ns.top_file_no()].clone(),
                    lookup: Lapper::new(intelligence.hovers),
                },
            );
            self.definitions.lock().await.insert(
                path,
                Definitions {
                    file: ns.files[ns.top_file_no()].clone(),
                    files: ns.files,
                    lookup: Lapper::new(intelligence.definitions),
                },
            );

            res.await;
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

    fn construct_builtins(bltn: &ast::Builtin, ns: &ast::Namespace) -> String {
        let mut msg = "[built-in] ".to_string();
        let prot = get_prototype(*bltn);

        if let Some(protval) = prot {
            for ret in &protval.ret {
                msg = format!("{} {}", msg, SolangServer::expanded_ty(ret, ns));
            }
            msg = format!("{} {} (", msg, protval.name);
            for arg in &protval.params {
                msg = format!("{}{}", msg, SolangServer::expanded_ty(arg, ns));
            }
            msg = format!("{}): {}", msg, protval.doc);
        }
        msg
    }

    // Constructs lookup table(messages) for the given statement by traversing the
    // statements and traversing inside the contents of the statements.
    fn construct_stmt(
        stmt: &ast::Statement,
        intelligence: &mut Intelligence,
        symtab: &symtable::Symtable,
        ns: &ast::Namespace,
    ) {
        match stmt {
            ast::Statement::Block { statements, .. } => {
                for stmt in statements {
                    SolangServer::construct_stmt(stmt, intelligence, symtab, ns);
                }
            }
            ast::Statement::VariableDecl(loc, var_no, param, expr) => {
                if let Some(exp) = expr {
                    SolangServer::construct_expr(exp, intelligence, symtab, ns);
                }
                let mut val = format!(
                    "{} {}",
                    SolangServer::expanded_ty(&param.ty, ns),
                    param.name_as_str()
                );
                if let Some(expr) = ns.var_constants.get(loc) {
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
                            write!(val, " = {}", n).unwrap();
                        }
                        _ => (),
                    }
                }

                if let Some(var) = symtab.vars.get(var_no) {
                    if var.slice {
                        val.push_str("\nreadonly: compiled to slice\n")
                    }
                }

                intelligence.hovers.push(HoverEntry {
                    start: param.loc.start(),
                    stop: param.loc.end(),
                    val,
                });

                let ty_decl_loc = SolangServer::type_declaration_loc(&param.ty, ns);
                if let Some(ty_loc) = param.ty_loc {
                    intelligence.definitions.push(DefinitionEntry {
                        start: ty_loc.start(),
                        stop: ty_loc.end(),
                        val: ty_decl_loc,
                    });
                }
            }
            ast::Statement::If(_locs, _, expr, stat1, stat2) => {
                SolangServer::construct_expr(expr, intelligence, symtab, ns);
                for st1 in stat1 {
                    SolangServer::construct_stmt(st1, intelligence, symtab, ns);
                }
                for st2 in stat2 {
                    SolangServer::construct_stmt(st2, intelligence, symtab, ns);
                }
            }
            ast::Statement::While(_locs, _blval, expr, stat1) => {
                SolangServer::construct_expr(expr, intelligence, symtab, ns);
                for st1 in stat1 {
                    SolangServer::construct_stmt(st1, intelligence, symtab, ns);
                }
            }
            ast::Statement::For {
                loc: _,
                reachable: _,
                init,
                cond,
                next,
                body,
            } => {
                if let Some(exp) = cond {
                    SolangServer::construct_expr(exp, intelligence, symtab, ns);
                }
                for stat in init {
                    SolangServer::construct_stmt(stat, intelligence, symtab, ns);
                }
                for stat in next {
                    SolangServer::construct_stmt(stat, intelligence, symtab, ns);
                }
                for stat in body {
                    SolangServer::construct_stmt(stat, intelligence, symtab, ns);
                }
            }
            ast::Statement::DoWhile(_locs, _blval, stat1, expr) => {
                SolangServer::construct_expr(expr, intelligence, symtab, ns);
                for st1 in stat1 {
                    SolangServer::construct_stmt(st1, intelligence, symtab, ns);
                }
            }
            ast::Statement::Expression(_locs, _, expr) => {
                SolangServer::construct_expr(expr, intelligence, symtab, ns);
            }
            ast::Statement::Delete(_locs, _typ, expr) => {
                SolangServer::construct_expr(expr, intelligence, symtab, ns);
            }
            ast::Statement::Destructure(_locs, _vecdestrfield, expr) => {
                SolangServer::construct_expr(expr, intelligence, symtab, ns);
                for vecstr in _vecdestrfield {
                    match vecstr {
                        ast::DestructureField::Expression(expr) => {
                            SolangServer::construct_expr(
                                expr,
                                intelligence,
                                symtab,
                                ns,
                            );
                        }
                        _ => continue,
                    }
                }
            }
            ast::Statement::Continue(_locs) => {}
            ast::Statement::Break(_) => {}
            ast::Statement::Return(_, None) => {}
            ast::Statement::Return(_, Some(expr)) => {
                SolangServer::construct_expr(expr, intelligence, symtab, ns);
            }
            ast::Statement::Emit {
                event_no,
                event_loc,
                args,
                ..
            } => {
                let event = &ns.events[*event_no];

                let mut val = render(&event.tags);

                write!(val, "```\nevent {} {{\n", event.symbol_name(ns)).unwrap();

                let mut iter = event.fields.iter().peekable();
                while let Some(field) = iter.next() {
                    writeln!(
                        val,
                        "\t{}{}{}{}",
                        field.ty.to_string(ns),
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

                intelligence.hovers.push(HoverEntry {
                    start: event_loc.start(),
                    stop: event_loc.end(),
                    val,
                });
                intelligence.definitions.push(DefinitionEntry {
                    start: event_loc.start(),
                    stop: event_loc.end(),
                    val: event.loc,
                });

                for arg in args {
                    SolangServer::construct_expr(arg, intelligence, symtab, ns);
                }
            }
            ast::Statement::TryCatch(_, _, try_stmt) => {
                SolangServer::construct_expr(
                    &try_stmt.expr,
                    intelligence,
                    symtab,
                    ns,
                );
                for vecstmt in &try_stmt.catch_stmt {
                    SolangServer::construct_stmt(vecstmt, intelligence, symtab, ns);
                }
                for vecstmt in &try_stmt.ok_stmt {
                    SolangServer::construct_stmt(vecstmt, intelligence, symtab, ns);
                }
                for okstmt in &try_stmt.errors {
                    for stmts in &okstmt.2 {
                        SolangServer::construct_stmt(stmts, intelligence, symtab, ns);
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
    fn construct_expr(
        expr: &ast::Expression,
        intelligence: &mut Intelligence,
        symtab: &symtable::Symtable,
        ns: &ast::Namespace,
    ) {
        match expr {
            // Variable types expression
            ast::Expression::BoolLiteral(locs, vl) => {
                let val = format!("(bool) {}", vl);
                intelligence.hovers.push(HoverEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val,
                });
            }
            ast::Expression::BytesLiteral(locs, typ, _vec_lst) => {
                let val = format!("({})", typ.to_string(ns));
                intelligence.hovers.push(HoverEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val,
                });
            }
            ast::Expression::CodeLiteral(locs, _val, _) => {
                let val = format!("({})", _val);
                intelligence.hovers.push(HoverEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val,
                });
            }
            ast::Expression::NumberLiteral(locs, typ, idx) => {
                intelligence.hovers.push(HoverEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val: typ.to_string(ns),
                });

                if let ast::Type::Enum(e) = typ {
                    if let Some(idx) = idx.to_usize() {
                        if let Some((_, val)) = &ns.enums[*e].values.get_index(idx) {
                            intelligence.definitions.push(DefinitionEntry {
                                start: locs.start(),
                                stop: locs.end(),
                                val: **val,
                            });
                        }
                    }
                }
            }
            ast::Expression::StructLiteral(locs, typ, expr) => {
                for expp in expr {
                    SolangServer::construct_expr(expp, intelligence, symtab, ns);
                }
                let ty_decl_loc = SolangServer::type_declaration_loc(typ, ns);
                intelligence.definitions.push(DefinitionEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val: ty_decl_loc,
                });
            }
            ast::Expression::ArrayLiteral(_locs, _, _arr, expr) => {
                for expp in expr {
                    SolangServer::construct_expr(expp, intelligence, symtab, ns);
                }
            }
            ast::Expression::ConstArrayLiteral(_locs, _, _arr, expr) => {
                for expp in expr {
                    SolangServer::construct_expr(expp, intelligence, symtab, ns);
                }
            }

            // Arithmetic expression
            ast::Expression::Add(locs, ty, unchecked, expr1, expr2) => {
                intelligence.hovers.push(HoverEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val: format!(
                        "{} {} addition",
                        if *unchecked { "unchecked " } else { "" },
                        ty.to_string(ns)
                    ),
                });

                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::Subtract(locs, ty, unchecked, expr1, expr2) => {
                intelligence.hovers.push(HoverEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val: format!(
                        "{} {} subtraction",
                        if *unchecked { "unchecked " } else { "" },
                        ty.to_string(ns)
                    ),
                });

                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::Multiply(locs, ty, unchecked, expr1, expr2) => {
                intelligence.hovers.push(HoverEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val: format!(
                        "{} {} multiply",
                        if *unchecked { "unchecked " } else { "" },
                        ty.to_string(ns)
                    ),
                });

                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::Divide(locs, ty, expr1, expr2) => {
                intelligence.hovers.push(HoverEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val: format!("{} divide", ty.to_string(ns)),
                });

                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::Modulo(locs, ty, expr1, expr2) => {
                intelligence.hovers.push(HoverEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val: format!("{} modulo", ty.to_string(ns)),
                });

                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::Power(locs, ty, unchecked, expr1, expr2) => {
                intelligence.hovers.push(HoverEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val: format!(
                        "{} {}power",
                        if *unchecked { "unchecked " } else { "" },
                        ty.to_string(ns)
                    ),
                });

                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }

            // Bitwise expresion
            ast::Expression::BitwiseOr(_locs, _typ, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::BitwiseAnd(_locs, _typ, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::BitwiseXor(_locs, _typ, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::ShiftLeft(_locs, _typ, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::ShiftRight(_locs, _typ, expr1, expr2, _bl) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }

            // Variable expression
            ast::Expression::Variable(loc, typ, var_no) => {
                let mut val = SolangServer::expanded_ty(typ, ns);

                if let Some(expr) = ns.var_constants.get(loc) {
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
                            write!(val, " {}", n).unwrap();
                        }
                        _ => (),
                    }
                }

                if let Some(var) = symtab.vars.get(var_no) {
                    if var.slice {
                        val.push_str("\nreadonly: compiles to slice\n")
                    }

                    intelligence.definitions.push(DefinitionEntry {
                        start: loc.start(),
                        stop: loc.end(),
                        val: var.id.loc,
                    });
                }

                intelligence.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.end(),
                    val,
                });
            }
            ast::Expression::ConstantVariable(locs, typ, contract_opt, var_no) => {
                let val = format!("constant ({})", SolangServer::expanded_ty(typ, ns,));
                intelligence.hovers.push(HoverEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val,
                });

                let val;
                if let Some(contract_no) = contract_opt {
                    val = ns.contracts[*contract_no].variables[*var_no].loc;
                } else {
                    val = ns.constants[*var_no].loc;
                }
                intelligence.definitions.push(DefinitionEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val,
                });
            }
            ast::Expression::StorageVariable(locs, typ, var_contract_no, var_no) => {
                let val = format!("({})", SolangServer::expanded_ty(typ, ns));
                intelligence.hovers.push(HoverEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val,
                });

                let store_var = &ns.contracts[*var_contract_no].variables[*var_no];
                intelligence.definitions.push(DefinitionEntry {
                    start: locs.start(),
                    stop: locs.end(),
                    val: store_var.loc,
                });
            }

            // Load expression
            ast::Expression::Load(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
            }
            ast::Expression::StorageLoad(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
            }
            ast::Expression::ZeroExt { expr, .. } => {
                SolangServer::construct_expr(expr, intelligence, symtab, ns);
            }
            ast::Expression::SignExt { expr, .. } => {
                SolangServer::construct_expr(expr, intelligence, symtab, ns);
            }
            ast::Expression::Trunc { expr, .. } => {
                SolangServer::construct_expr(expr, intelligence, symtab, ns);
            }
            ast::Expression::Cast { expr, .. } => {
                SolangServer::construct_expr(expr, intelligence, symtab, ns);
            }
            ast::Expression::BytesCast { expr, .. } => {
                SolangServer::construct_expr(expr, intelligence, symtab, ns);
            }

            //Increment-Decrement expression
            ast::Expression::PreIncrement(_locs, _typ, _, expr1) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
            }
            ast::Expression::PreDecrement(_locs, _typ, _, expr1) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
            }
            ast::Expression::PostIncrement(_locs, _typ, _, expr1) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
            }
            ast::Expression::PostDecrement(_locs, _typ, _, expr1) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
            }
            ast::Expression::Assign(_locs, _typ, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }

            // Compare expression
            ast::Expression::More(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::Less(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::MoreEqual(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::LessEqual(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::Equal(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::NotEqual(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }

            ast::Expression::Not(_locs, expr1) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
            }
            ast::Expression::Complement(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
            }
            ast::Expression::UnaryMinus(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
            }

            ast::Expression::ConditionalOperator(_locs, _typ, expr1, expr2, expr3) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
                SolangServer::construct_expr(expr3, intelligence, symtab, ns);
            }

            ast::Expression::Subscript(_locs, _, _, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }

            ast::Expression::StructMember(_locs, _typ, expr1, _val) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
            }

            // Array operation expression
            ast::Expression::AllocDynamicBytes(_locs, _typ, expr1, _valvec) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
            }
            ast::Expression::StorageArrayLength { array, .. } => {
                SolangServer::construct_expr(array, intelligence, symtab, ns);
            }

            // String operations expression
            ast::Expression::StringCompare(_locs, _strloc1, _strloc2) => {
                if let ast::StringLocation::RunTime(expr1) = _strloc1 {
                    SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                }
                if let ast::StringLocation::RunTime(expr2) = _strloc1 {
                    SolangServer::construct_expr(expr2, intelligence, symtab, ns);
                }
            }
            ast::Expression::StringConcat(_locs, _typ, _strloc1, _strloc2) => {
                if let ast::StringLocation::RunTime(expr1) = _strloc1 {
                    SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                }
                if let ast::StringLocation::RunTime(expr2) = _strloc1 {
                    SolangServer::construct_expr(expr2, intelligence, symtab, ns);
                }
            }

            ast::Expression::Or(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }
            ast::Expression::And(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, intelligence, symtab, ns);
                SolangServer::construct_expr(expr2, intelligence, symtab, ns);
            }

            // Function call expression
            ast::Expression::InternalFunctionCall {
                loc,
                function,
                args,
                ..
            } => {
                if let ast::Expression::InternalFunction { function_no, .. } = function.as_ref() {
                    let fnc = &ns.functions[*function_no];
                    let msg_tg = render(&fnc.tags[..]);

                    let mut val = format!("{} \n\n {} {}(", msg_tg, fnc.ty, fnc.name);

                    for parm in &*fnc.params {
                        let msg = format!(
                            "{}:{}, \n\n",
                            parm.name_as_str(),
                            SolangServer::expanded_ty(&parm.ty, ns)
                        );
                        val = format!("{} {}", val, msg);
                    }

                    val = format!("{} ) returns (", val);

                    for ret in &*fnc.returns {
                        let msg = format!(
                            "{}:{}, ",
                            ret.name_as_str(),
                            SolangServer::expanded_ty(&ret.ty, ns)
                        );
                        val = format!("{} {}", val, msg);
                    }

                    val = format!("{})", val);
                    intelligence.hovers.push(HoverEntry {
                        start: loc.start(),
                        stop: loc.end(),
                        val,
                    });
                    intelligence.definitions.push(DefinitionEntry {
                        start: loc.start(),
                        stop: loc.end(),
                        val: fnc.loc,
                    });
                }

                for arg in args {
                    SolangServer::construct_expr(arg, intelligence, symtab, ns);
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
                    let fnc = &ns.functions[*function_no];
                    let msg_tg = render(&fnc.tags[..]);
                    let mut val = format!("{} \n\n {} {}(", msg_tg, fnc.ty, fnc.name);

                    for parm in &*fnc.params {
                        let msg = format!(
                            "{}:{}, \n\n",
                            parm.name_as_str(),
                            SolangServer::expanded_ty(&parm.ty, ns)
                        );
                        val = format!("{} {}", val, msg);
                    }

                    val = format!("{} ) \n\n returns (", val);

                    for ret in &*fnc.returns {
                        let msg = format!(
                            "{}:{}, ",
                            ret.name_as_str(),
                            SolangServer::expanded_ty(&ret.ty, ns)
                        );
                        val = format!("{} {}", val, msg);
                    }

                    val = format!("{})", val);
                    intelligence.hovers.push(HoverEntry {
                        start: loc.start(),
                        stop: loc.end(),
                        val,
                    });
                    intelligence.definitions.push(DefinitionEntry {
                        start: loc.start(),
                        stop: loc.end(),
                        val: fnc.loc,
                    });

                    SolangServer::construct_expr(address, intelligence, symtab, ns);
                    for expp in args {
                        SolangServer::construct_expr(expp, intelligence, symtab, ns);
                    }
                    if let Some(value) = &call_args.value {
                        SolangServer::construct_expr(value, intelligence, symtab, ns);
                    }
                    if let Some(gas) = &call_args.gas {
                        SolangServer::construct_expr(gas, intelligence, symtab, ns);
                    }
                }
            }
            ast::Expression::ExternalFunctionCallRaw {
                address,
                args,
                call_args,
                ..
            } => {
                SolangServer::construct_expr(args, intelligence, symtab, ns);
                SolangServer::construct_expr(address, intelligence, symtab, ns);
                if let Some(value) = &call_args.value {
                    SolangServer::construct_expr(value, intelligence, symtab, ns);
                }
                if let Some(gas) = &call_args.gas {
                    SolangServer::construct_expr(gas, intelligence, symtab, ns);
                }
            }
            ast::Expression::Constructor {
                loc,
                contract_no,
                constructor_no: _,
                args,
                call_args,
            } => {
                if let Some(gas) = &call_args.gas {
                    SolangServer::construct_expr(gas, intelligence, symtab, ns);
                }
                for expp in args {
                    SolangServer::construct_expr(expp, intelligence, symtab, ns);
                }
                if let Some(optval) = &call_args.value {
                    SolangServer::construct_expr(optval, intelligence, symtab, ns);
                }
                if let Some(optsalt) = &call_args.salt {
                    SolangServer::construct_expr(optsalt, intelligence, symtab, ns);
                }
                if let Some(address) = &call_args.address {
                    SolangServer::construct_expr(address, intelligence, symtab, ns);
                }
                if let Some(seeds) = &call_args.seeds {
                    SolangServer::construct_expr(seeds, intelligence, symtab, ns);
                }

                let c = &ns.contracts[*contract_no];
                intelligence.definitions.push(DefinitionEntry {
                    start: loc.start(),
                    stop: loc.end(),
                    val: c.loc,
                });
            }
            ast::Expression::Builtin(_locs, _typ, _builtin, expr) => {
                let val = SolangServer::construct_builtins(_builtin, ns);
                intelligence.hovers.push(HoverEntry {
                    start: _locs.start(),
                    stop: _locs.end(),
                    val,
                });
                for expp in expr {
                    SolangServer::construct_expr(expp, intelligence, symtab, ns);
                }
            }
            ast::Expression::FormatString(_, sections) => {
                for (_, e) in sections {
                    SolangServer::construct_expr(e, intelligence, symtab, ns);
                }
            }
            ast::Expression::List(_locs, expr) => {
                for expp in expr {
                    SolangServer::construct_expr(expp, intelligence, symtab, ns);
                }
            }
            _ => {}
        }
    }

    // Constructs contract fields and stores it in the lookup table.
    fn construct_cont(
        contvar: &ast::Variable,
        intelligence: &mut Intelligence,
        samptb: &symtable::Symtable,
        ns: &ast::Namespace,
    ) {
        let val = format!(
            "{} {}",
            SolangServer::expanded_ty(&contvar.ty, ns),
            contvar.name
        );
        intelligence.hovers.push(HoverEntry {
            start: contvar.loc.start(),
            stop: contvar.loc.end(),
            val,
        });
        if let Some(expr) = &contvar.initializer {
            SolangServer::construct_expr(expr, intelligence, samptb, ns);
        }
    }

    // Constructs struct fields and stores it in the lookup table.
    fn construct_strct(
        strfld: &ast::Parameter,
        intelligence: &mut Intelligence,
        ns: &ast::Namespace,
    ) {
        let val = format!("{} {}", strfld.ty.to_string(ns), strfld.name_as_str());
        intelligence.hovers.push(HoverEntry {
            start: strfld.loc.start(),
            stop: strfld.loc.end(),
            val,
        });

        let ty_decl_loc = SolangServer::type_declaration_loc(&strfld.ty, ns);
        if let Some(ty_loc) = strfld.ty_loc {
            intelligence.definitions.push(DefinitionEntry {
                start: ty_loc.start(),
                stop: ty_loc.end(),
                val: ty_decl_loc,
            });
        }
    }

    // Traverses namespace to build messages stored in the lookup table for hover feature.
    fn traverse(
        ns: &ast::Namespace,
        intelligence: &mut Intelligence
    ) {
        for enm in &ns.enums {
            for (idx, (nam, loc)) in enm.values.iter().enumerate() {
                let val = format!("{} {}, \n\n", nam, idx);
                intelligence.hovers.push(HoverEntry {
                    start: loc.start(),
                    stop: loc.end(),
                    val,
                });
            }

            let val = render(&enm.tags[..]);
            intelligence.hovers.push(HoverEntry {
                start: enm.loc.start(),
                stop: enm.loc.start() + enm.name.len(),
                val,
            });
        }

        for strct in &ns.structs {
            if let pt::Loc::File(_, start, _) = &strct.loc {
                for filds in &strct.fields {
                    SolangServer::construct_strct(filds, intelligence, ns);
                }

                let val = render(&strct.tags[..]);
                intelligence.hovers.push(HoverEntry {
                    start: *start,
                    stop: start + strct.name.len(),
                    val,
                });
            }
        }

        for func in &ns.functions {
            if func.is_accessor || func.loc == pt::Loc::Builtin {
                // accessor functions are synthetic; ignore them, all the locations are fake
                continue;
            }

            for note in &func.annotations {
                match note {
                    ast::ConstructorAnnotation::Bump(expr)
                    | ast::ConstructorAnnotation::Seed(expr)
                    | ast::ConstructorAnnotation::Payer(expr)
                    | ast::ConstructorAnnotation::Space(expr) => SolangServer::construct_expr(
                        expr,
                        intelligence,
                        &func.symtable,
                        ns,
                    ),
                }
            }

            for parm in &*func.params {
                let val = SolangServer::expanded_ty(&parm.ty, ns);
                intelligence.hovers.push(HoverEntry {
                    start: parm.loc.start(),
                    stop: parm.loc.end(),
                    val,
                });

                let ty_decl_loc = SolangServer::type_declaration_loc(&parm.ty, ns);
                if let Some(ty_loc) = parm.ty_loc {
                    intelligence.definitions.push(DefinitionEntry {
                        start: ty_loc.start(),
                        stop: ty_loc.end(),
                        val: ty_decl_loc,
                    });
                }
            }

            for ret in &*func.returns {
                let val = SolangServer::expanded_ty(&ret.ty, ns);
                intelligence.hovers.push(HoverEntry {
                    start: ret.loc.start(),
                    stop: ret.loc.end(),
                    val,
                });

                let ty_decl_loc = SolangServer::type_declaration_loc(&ret.ty, ns);
                if let Some(ty_loc) = ret.ty_loc {
                    intelligence.definitions.push(DefinitionEntry {
                        start: ty_loc.start(),
                        stop: ty_loc.end(),
                        val: ty_decl_loc,
                    });
                }
            }

            for stmt in &func.body {
                SolangServer::construct_stmt(stmt, intelligence, &func.symtable, ns);
            }
        }

        for constant in &ns.constants {
            let samptb = symtable::Symtable::new();
            SolangServer::construct_cont(constant, intelligence, &samptb, ns);

            let val = render(&constant.tags[..]);
            intelligence.hovers.push(HoverEntry {
                start: constant.loc.start(),
                stop: constant.loc.start() + constant.name.len(),
                val,
            });

            let ty_decl_loc = SolangServer::type_declaration_loc(&constant.ty, ns);
            intelligence.definitions.push(DefinitionEntry {
                start: constant.loc.start(),
                stop: constant.loc.start() + constant.name.len(),
                val: ty_decl_loc,
            });
        }

        for contrct in &ns.contracts {
            let val = render(&contrct.tags[..]);
            intelligence.hovers.push(HoverEntry {
                start: contrct.loc.start(),
                stop: contrct.loc.start() + val.len(),
                val,
            });

            for varscont in &contrct.variables {
                let samptb = symtable::Symtable::new();
                SolangServer::construct_cont(varscont, intelligence, &samptb, ns);

                let val = render(&varscont.tags[..]);
                intelligence.hovers.push(HoverEntry {
                    start: varscont.loc.start(),
                    stop: varscont.loc.start() + varscont.name.len(),
                    val,
                });
                let ty_decl_loc = SolangServer::type_declaration_loc(&varscont.ty, ns);
                intelligence.definitions.push(DefinitionEntry {
                    start: varscont.loc.start(),
                    stop: varscont.loc.start() + varscont.name.len(),
                    val: ty_decl_loc,
                });
            }
        }

        for entdcl in &ns.events {
            for filds in &entdcl.fields {
                SolangServer::construct_strct(filds, intelligence, ns);
            }
            let val = render(&entdcl.tags[..]);
            intelligence.hovers.push(HoverEntry {
                start: entdcl.loc.start(),
                stop: entdcl.loc.start() + entdcl.name.len(),
                val,
            });
        }

        for lookup in intelligence.hovers.iter_mut() {
            if let Some(msg) = ns
                .hover_overrides
                .get(&pt::Loc::File(0, lookup.start, lookup.stop))
            {
                lookup.val = msg.clone();
            }
        }
    }

    fn type_declaration_loc(ty: &ast::Type, ns: &ast::Namespace) -> pt::Loc {
        match ty {
            ast::Type::Ref(ty) => SolangServer::type_declaration_loc(ty, ns),
            ast::Type::StorageRef(_, ty) => SolangServer::type_declaration_loc(ty, ns),
            ast::Type::Struct(struct_type) => struct_type.definition(ns).loc,
            ast::Type::Enum(n) => ns.enums[*n].loc,
            ast::Type::Contract(n) => ns.contracts[*n].loc,
            _ => pt::Loc::Builtin,
        }
    }

    /// Render the type with struct/enum fields expanded
    fn expanded_ty(ty: &ast::Type, ns: &ast::Namespace) -> String {
        match ty {
            ast::Type::Ref(ty) => SolangServer::expanded_ty(ty, ns),
            ast::Type::StorageRef(_, ty) => SolangServer::expanded_ty(ty, ns),
            ast::Type::Struct(struct_type) => {
                let strct = struct_type.definition(ns);

                let mut msg = render(&strct.tags);

                writeln!(msg, "```\nstruct {} {{", strct).unwrap();

                let mut iter = strct.fields.iter().peekable();
                while let Some(field) = iter.next() {
                    writeln!(
                        msg,
                        "\t{} {}{}",
                        field.ty.to_string(ns),
                        field.name_as_str(),
                        if iter.peek().is_some() { "," } else { "" }
                    )
                    .unwrap();
                }

                msg.push_str("};\n```\n");

                msg
            }
            ast::Type::Enum(n) => {
                let enm = &ns.enums[*n];

                let mut msg = render(&enm.tags);

                write!(msg, "```\nenum {} {{\n", enm).unwrap();

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
            _ => ty.to_string(ns),
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
                definition_provider: Some(OneOf::Left(true)),
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
                    .lookup
                    .find(offset, offset)
                    .min_by(|a, b| (a.stop - a.start).cmp(&(b.stop - b.start)))
                {
                    let loc = pt::Loc::File(0, hover.start, hover.stop);
                    let range = SolangServer::loc_to_range(&loc, &hovers.file);

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

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let txtdoc = params.text_document_position_params.text_document;
        let pos = params.text_document_position_params.position;

        let uri = txtdoc.uri;

        if let Ok(path) = uri.to_file_path() {
            let definitions = self.definitions.lock().await;
            if let Some(defs) = definitions.get(&path) {
                let offset = defs
                    .file
                    .get_offset(pos.line as usize, pos.character as usize);

                // The shortest definition for the position will be most informative
                if let Some(definition) = defs
                    .lookup
                    .find(offset, offset)
                    .min_by(|a, b| (a.stop - a.start).cmp(&(b.stop - b.start)))
                {
                    if let pt::Loc::File(file_no, _, _) = definition.val {
                        let range =
                            SolangServer::loc_to_range(&definition.val, &defs.files[file_no]);

                        return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                            uri: Url::from_file_path(&defs.files[file_no].path).unwrap(),
                            range,
                        })));
                    }
                }
            }
        }

        Ok(None)
    }
}
