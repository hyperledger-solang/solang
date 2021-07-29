use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tower_lsp::{LspService, Server};

use solang::codegen::codegen;
use solang::file_cache::FileCache;
use solang::parse_and_resolve;
use solang::parser::pt;
use solang::Target;

use solang::sema::*;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, HoverProviderCapability, Position, Range,
};

use solang::*;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::Mutex;

use solang::sema::ast::*;
use solang::sema::tags::*;

use solang::sema::builtin::get_prototype;

pub struct Hovers {
    offsets: sema::diagnostics::FileOffsets,
    lookup: Vec<(usize, usize, String)>,
}

pub struct SolangServer {
    client: Client,
    target: Target,
    files: Mutex<HashMap<PathBuf, Hovers>>,
}

#[tokio::main(flavor = "current_thread")]
pub async fn start_server(target: Target) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, messages) = LspService::new(|client| SolangServer {
        client,
        target,
        files: Mutex::new(HashMap::new()),
    });

    Server::new(stdin, stdout)
        .interleave(messages)
        .serve(service)
        .await;

    std::process::exit(1);
}

impl SolangServer {
    /// Parse file
    async fn parse_file(&self, uri: Url) {
        if let Ok(path) = uri.to_file_path() {
            let mut filecache = FileCache::new();

            let dir = path.parent().unwrap();

            if let Ok(dir) = dir.canonicalize() {
                filecache.add_import_path(dir);
            }

            let os_str = path.file_name().unwrap();

            let mut ns = parse_and_resolve(os_str.to_str().unwrap(), &mut filecache, self.target);

            // codegen all the contracts; some additional errors/warnings will be detected here
            for contract_no in 0..ns.contracts.len() {
                codegen(contract_no, &mut ns, &Default::default());
            }

            let offsets = ns.file_offset(&mut filecache);

            let diags = ns
                .diagnostics
                .iter()
                .filter_map(|diag| {
                    let pos = diag.pos.unwrap();

                    if pos.0 != 0 {
                        // The first file is the one we wanted to parse; others are imported
                        return None;
                    }

                    let related_information = if diag.notes.is_empty() {
                        None
                    } else {
                        Some(
                            diag.notes
                                .iter()
                                .map(|note| DiagnosticRelatedInformation {
                                    message: note.message.to_string(),
                                    location: Location {
                                        uri: Url::from_file_path(&ns.files[note.pos.0]).unwrap(),
                                        range: SolangServer::loc_to_range(&note.pos, &offsets),
                                    },
                                })
                                .collect(),
                        )
                    };

                    let sev = match diag.level {
                        ast::Level::Info => DiagnosticSeverity::Information,
                        ast::Level::Warning => DiagnosticSeverity::Warning,
                        ast::Level::Error => DiagnosticSeverity::Error,
                        ast::Level::Debug => {
                            return None;
                        }
                    };

                    let range = SolangServer::loc_to_range(&pos, &offsets);

                    Some(Diagnostic {
                        range,
                        message: diag.message.to_string(),
                        severity: Some(sev),
                        source: None,
                        code: None,
                        code_description: None,
                        related_information,
                        tags: None,
                        data: None,
                    })
                })
                .collect();

            let res = self.client.publish_diagnostics(uri, diags, None);

            let mut lookup: Vec<(usize, usize, String)> = Vec::new();
            let mut fnc_map: HashMap<String, String> = HashMap::new();

            SolangServer::traverse(&ns, &mut lookup, &mut fnc_map);

            lookup.sort_by_key(|k| k.0);

            self.files
                .lock()
                .await
                .insert(path, Hovers { offsets, lookup });

            res.await;
        }
    }

    /// Calculate the line and column from the Loc offset received from the parser
    fn loc_to_range(loc: &pt::Loc, file_offsets: &diagnostics::FileOffsets) -> Range {
        let (line, column) = file_offsets.convert(loc.0, loc.1);
        let start = Position::new(line as u32, column as u32);
        let (line, column) = file_offsets.convert(loc.0, loc.2);
        let end = Position::new(line as u32, column as u32);

        Range::new(start, end)
    }

    fn construct_builtins(bltn: &sema::ast::Builtin, ns: &ast::Namespace) -> String {
        let mut msg = "[built-in] ".to_string();
        let prot = get_prototype(*bltn);

        if let Some(protval) = prot {
            for ret in protval.ret {
                msg = format!("{} {}", msg, SolangServer::expanded_ty(ret, ns));
            }
            msg = format!("{} {} (", msg, protval.name);
            for arg in protval.args {
                msg = format!("{}{}", msg, SolangServer::expanded_ty(arg, ns));
            }
            msg = format!("{}): {}", msg, protval.doc.to_string());
        }
        msg
    }

    // Constructs lookup table(messages) for the given statement by traversing the
    // statements and traversing inside the contents of the statements.
    fn construct_stmt(
        stmt: &Statement,
        lookup_tbl: &mut Vec<(usize, usize, String)>,
        symtab: &sema::symtable::Symtable,
        fnc_map: &HashMap<String, String>,
        ns: &ast::Namespace,
    ) {
        match stmt {
            Statement::VariableDecl(loc, var_no, _param, expr) => {
                if let Some(exp) = expr {
                    SolangServer::construct_expr(exp, lookup_tbl, symtab, fnc_map, ns);
                }
                let mut msg = SolangServer::expanded_ty(&_param.ty, ns);
                msg = format!("{} {}", msg, _param.name);
                if let Some(expr) = ns.var_constants.get(loc) {
                    match expr {
                        Expression::BytesLiteral(_, ast::Type::Bytes(_), bs)
                        | Expression::BytesLiteral(_, ast::Type::DynamicBytes, bs) => {
                            msg.push_str(&format!(" = hex\"{}\"", hex::encode(&bs)));
                        }
                        Expression::BytesLiteral(_, ast::Type::String, bs) => {
                            msg.push_str(&format!(" = \"{}\"", String::from_utf8_lossy(bs)));
                        }
                        Expression::NumberLiteral(_, ast::Type::Uint(_), n)
                        | Expression::NumberLiteral(_, ast::Type::Int(_), n) => {
                            msg.push_str(&format!(" = {}", n));
                        }
                        _ => (),
                    }
                }

                if let Some(var) = symtab.vars.get(var_no) {
                    if var.slice {
                        msg.push_str("\nreadonly: compiled to slice\n")
                    }
                }

                lookup_tbl.push((_param.loc.1, _param.loc.2, msg));
            }
            Statement::If(_locs, _, expr, stat1, stat2) => {
                SolangServer::construct_expr(expr, lookup_tbl, symtab, fnc_map, ns);
                for st1 in stat1 {
                    SolangServer::construct_stmt(st1, lookup_tbl, symtab, fnc_map, ns);
                }
                for st2 in stat2 {
                    SolangServer::construct_stmt(st2, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            Statement::While(_locs, _blval, expr, stat1) => {
                SolangServer::construct_expr(expr, lookup_tbl, symtab, fnc_map, ns);
                for st1 in stat1 {
                    SolangServer::construct_stmt(st1, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            Statement::For {
                loc: _,
                reachable: _,
                init,
                cond,
                next,
                body,
            } => {
                if let Some(exp) = cond {
                    SolangServer::construct_expr(exp, lookup_tbl, symtab, fnc_map, ns);
                }
                for stat in init {
                    SolangServer::construct_stmt(stat, lookup_tbl, symtab, fnc_map, ns);
                }
                for stat in next {
                    SolangServer::construct_stmt(stat, lookup_tbl, symtab, fnc_map, ns);
                }
                for stat in body {
                    SolangServer::construct_stmt(stat, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            Statement::DoWhile(_locs, _blval, stat1, expr) => {
                SolangServer::construct_expr(expr, lookup_tbl, symtab, fnc_map, ns);
                for st1 in stat1 {
                    SolangServer::construct_stmt(st1, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            Statement::Expression(_locs, _, expr) => {
                SolangServer::construct_expr(expr, lookup_tbl, symtab, fnc_map, ns);
            }
            Statement::Delete(_locs, _typ, expr) => {
                SolangServer::construct_expr(expr, lookup_tbl, symtab, fnc_map, ns);
            }
            Statement::Destructure(_locs, _vecdestrfield, expr) => {
                SolangServer::construct_expr(expr, lookup_tbl, symtab, fnc_map, ns);
                for vecstr in _vecdestrfield {
                    match vecstr {
                        DestructureField::Expression(expr) => {
                            SolangServer::construct_expr(expr, lookup_tbl, symtab, fnc_map, ns);
                        }
                        _ => continue,
                    }
                }
            }
            Statement::Continue(_locs) => {}
            Statement::Break(_) => {}
            Statement::Return(_locs, expr) => {
                for expp in expr {
                    SolangServer::construct_expr(expp, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            Statement::Emit {
                event_no,
                event_loc,
                args,
                ..
            } => {
                let event = &ns.events[*event_no];

                let mut msg = render(&event.tags);

                msg.push_str(&format!("```\nevent {} {{\n", event.symbol_name(ns)));

                let mut iter = event.fields.iter().peekable();
                while let Some(field) = iter.next() {
                    msg.push_str(&format!(
                        "\t{}{}{}{}\n",
                        field.ty.to_string(ns),
                        if field.indexed { " indexed " } else { " " },
                        field.name,
                        if iter.peek().is_some() { "," } else { "" }
                    ));
                }

                msg.push_str(&format!(
                    "}}{};\n```\n",
                    if event.anonymous { " anonymous" } else { "" }
                ));

                lookup_tbl.push((event_loc.1, event_loc.2, msg));

                for arg in args {
                    SolangServer::construct_expr(arg, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            Statement::TryCatch {
                loc: _,
                reachable: _,
                expr,
                returns: _,
                ok_stmt,
                error,
                catch_param: _,
                catch_param_pos: _,
                catch_stmt,
            } => {
                SolangServer::construct_expr(expr, lookup_tbl, symtab, fnc_map, ns);
                for vecstmt in catch_stmt {
                    SolangServer::construct_stmt(vecstmt, lookup_tbl, symtab, fnc_map, ns);
                }
                for vecstmt in ok_stmt {
                    SolangServer::construct_stmt(vecstmt, lookup_tbl, symtab, fnc_map, ns);
                }
                if let Some(okstmt) = error {
                    for stmts in &okstmt.2 {
                        SolangServer::construct_stmt(stmts, lookup_tbl, symtab, fnc_map, ns);
                    }
                }
            }
            Statement::Underscore(_loc) => {}
        }
    }

    // Constructs lookup table(messages) by traversing over the expressions and storing
    // the respective expression type messages in the table.
    fn construct_expr(
        expr: &Expression,
        lookup_tbl: &mut Vec<(usize, usize, String)>,
        symtab: &sema::symtable::Symtable,
        fnc_map: &HashMap<String, String>,
        ns: &ast::Namespace,
    ) {
        match expr {
            Expression::FunctionArg(locs, typ, _sample_sz) => {
                let msg = SolangServer::expanded_ty(typ, ns);
                lookup_tbl.push((locs.1, locs.2, msg));
            }

            // Variable types expression
            Expression::BoolLiteral(locs, vl) => {
                let msg = format!("(bool) {}", vl);
                lookup_tbl.push((locs.1, locs.2, msg));
            }
            Expression::BytesLiteral(locs, typ, _vec_lst) => {
                let msg = format!("({})", typ.to_string(ns));
                lookup_tbl.push((locs.1, locs.2, msg));
            }
            Expression::CodeLiteral(locs, _val, _) => {
                let msg = format!("({})", _val);
                lookup_tbl.push((locs.1, locs.2, msg));
            }
            Expression::NumberLiteral(locs, typ, _) => {
                let msg = typ.to_string(ns);
                lookup_tbl.push((locs.1, locs.2, msg));
            }
            Expression::StructLiteral(_locs, _typ, expr) => {
                for expp in expr {
                    SolangServer::construct_expr(expp, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            Expression::ArrayLiteral(_locs, _, _arr, expr) => {
                for expp in expr {
                    SolangServer::construct_expr(expp, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            Expression::ConstArrayLiteral(_locs, _, _arr, expr) => {
                for expp in expr {
                    SolangServer::construct_expr(expp, lookup_tbl, symtab, fnc_map, ns);
                }
            }

            // Arithmetic expression
            Expression::Add(locs, ty, expr1, expr2) => {
                lookup_tbl.push((locs.1, locs.2, format!("{} addition", ty.to_string(ns))));

                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::Subtract(locs, ty, expr1, expr2) => {
                lookup_tbl.push((locs.1, locs.2, format!("{} subtraction", ty.to_string(ns))));

                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::Multiply(locs, ty, expr1, expr2) => {
                lookup_tbl.push((locs.1, locs.2, format!("{} multiply", ty.to_string(ns))));

                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::Divide(locs, ty, expr1, expr2) => {
                lookup_tbl.push((locs.1, locs.2, format!("{} divide", ty.to_string(ns))));

                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::Modulo(locs, ty, expr1, expr2) => {
                lookup_tbl.push((locs.1, locs.2, format!("{} modulo", ty.to_string(ns))));

                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::Power(locs, ty, expr1, expr2) => {
                lookup_tbl.push((locs.1, locs.2, format!("{} power", ty.to_string(ns))));

                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }

            // Bitwise expresion
            Expression::BitwiseOr(_locs, _typ, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::BitwiseAnd(_locs, _typ, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::BitwiseXor(_locs, _typ, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::ShiftLeft(_locs, _typ, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::ShiftRight(_locs, _typ, expr1, expr2, _bl) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }

            // Variable expression
            Expression::Variable(loc, typ, var_no) => {
                let mut msg = SolangServer::expanded_ty(typ, ns);

                if let Some(expr) = ns.var_constants.get(loc) {
                    match expr {
                        Expression::BytesLiteral(_, ast::Type::Bytes(_), bs)
                        | Expression::BytesLiteral(_, ast::Type::DynamicBytes, bs) => {
                            msg.push_str(&format!(" hex\"{}\"", hex::encode(&bs)));
                        }
                        Expression::BytesLiteral(_, ast::Type::String, bs) => {
                            msg.push_str(&format!(" \"{}\"", String::from_utf8_lossy(bs)));
                        }
                        Expression::NumberLiteral(_, ast::Type::Uint(_), n)
                        | Expression::NumberLiteral(_, ast::Type::Int(_), n) => {
                            msg.push_str(&format!(" {}", n));
                        }
                        _ => (),
                    }
                }

                if let Some(var) = symtab.vars.get(var_no) {
                    if var.slice {
                        msg.push_str("\nreadonly: compiles to slice\n")
                    }
                }

                lookup_tbl.push((loc.1, loc.2, msg));
            }
            Expression::ConstantVariable(locs, typ, _val1, _val2) => {
                let msg = format!("constant ({})", SolangServer::expanded_ty(typ, ns,));
                lookup_tbl.push((locs.1, locs.2, msg));
            }
            Expression::StorageVariable(locs, typ, _val1, _val2) => {
                let msg = format!("({})", SolangServer::expanded_ty(typ, ns));
                lookup_tbl.push((locs.1, locs.2, msg));
            }

            // Load expression
            Expression::Load(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::StorageLoad(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::ZeroExt(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::SignExt(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::Trunc(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::Cast(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::BytesCast(_loc, _typ1, _typ2, expr) => {
                SolangServer::construct_expr(expr, lookup_tbl, symtab, fnc_map, ns);
            }

            //Increment-Decrement expression
            Expression::PreIncrement(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::PreDecrement(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::PostIncrement(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::PostDecrement(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::Assign(_locs, _typ, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }

            // Compare expression
            Expression::More(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::Less(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::MoreEqual(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::LessEqual(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::Equal(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::NotEqual(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }

            Expression::Not(_locs, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::Complement(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::UnaryMinus(_locs, _typ, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }

            Expression::Ternary(_locs, _typ, expr1, expr2, expr3) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr3, lookup_tbl, symtab, fnc_map, ns);
            }

            Expression::Subscript(_locs, _typ, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }

            Expression::StructMember(_locs, _typ, expr1, _val) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }

            // Array operation expression
            Expression::AllocDynamicArray(_locs, _typ, expr1, _valvec) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::DynamicArrayLength(_locs, expr1) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::DynamicArraySubscript(_locs, _typ, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::DynamicArrayPush(_locs, expr1, _typ, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::DynamicArrayPop(_locs, expr1, _typ) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::StorageBytesSubscript(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::StorageArrayLength { array, .. } => {
                SolangServer::construct_expr(array, lookup_tbl, symtab, fnc_map, ns);
            }

            //String operations expression
            Expression::StringCompare(_locs, _strloc1, _strloc2) => {
                if let StringLocation::RunTime(expr1) = _strloc1 {
                    SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                }
                if let StringLocation::RunTime(expr2) = _strloc1 {
                    SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            Expression::StringConcat(_locs, _typ, _strloc1, _strloc2) => {
                if let StringLocation::RunTime(expr1) = _strloc1 {
                    SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                }
                if let StringLocation::RunTime(expr2) = _strloc1 {
                    SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
                }
            }

            Expression::Or(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::And(_locs, expr1, expr2) => {
                SolangServer::construct_expr(expr1, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(expr2, lookup_tbl, symtab, fnc_map, ns);
            }

            // Function call expression
            Expression::InternalFunctionCall {
                loc,
                function,
                args,
                ..
            } => {
                if let Expression::InternalFunction { function_no, .. } = function.as_ref() {
                    let fnc = &ns.functions[*function_no];
                    let msg_tg = render(&fnc.tags[..]);

                    let mut param_msg = format!("{} \n\n {} {}(", msg_tg, fnc.ty, fnc.name);

                    for parm in &fnc.params {
                        let msg = format!(
                            "{}:{}, \n\n",
                            parm.name,
                            SolangServer::expanded_ty(&parm.ty, ns)
                        );
                        param_msg = format!("{} {}", param_msg, msg);
                    }

                    param_msg = format!("{} ) returns (", param_msg);

                    for ret in &fnc.returns {
                        let msg =
                            format!("{}:{}, ", ret.name, SolangServer::expanded_ty(&ret.ty, ns));
                        param_msg = format!("{} {}", param_msg, msg);
                    }

                    param_msg = format!("{})", param_msg);
                    lookup_tbl.push((loc.1, loc.2, param_msg));
                }

                for arg in args {
                    SolangServer::construct_expr(arg, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            Expression::ExternalFunctionCall {
                loc,
                function,
                args,
                value,
                gas,
                ..
            } => {
                if let Expression::ExternalFunction {
                    function_no,
                    address,
                    ..
                } = function.as_ref()
                {
                    // modifiers do not have mutability, bases or modifiers itself
                    let fnc = &ns.functions[*function_no];
                    let msg_tg = render(&fnc.tags[..]);
                    let mut param_msg = format!("{} \n\n {} {}(", msg_tg, fnc.ty, fnc.name);

                    for parm in &fnc.params {
                        let msg = format!(
                            "{}:{}, \n\n",
                            parm.name,
                            SolangServer::expanded_ty(&parm.ty, ns)
                        );
                        param_msg = format!("{} {}", param_msg, msg);
                    }

                    param_msg = format!("{} ) \n\n returns (", param_msg);

                    for ret in &fnc.returns {
                        let msg =
                            format!("{}:{}, ", ret.name, SolangServer::expanded_ty(&ret.ty, ns));
                        param_msg = format!("{} {}", param_msg, msg);
                    }

                    param_msg = format!("{})", param_msg);
                    lookup_tbl.push((loc.1, loc.2, param_msg));

                    SolangServer::construct_expr(address, lookup_tbl, symtab, fnc_map, ns);
                    for expp in args {
                        SolangServer::construct_expr(expp, lookup_tbl, symtab, fnc_map, ns);
                    }

                    SolangServer::construct_expr(value, lookup_tbl, symtab, fnc_map, ns);
                    SolangServer::construct_expr(gas, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            Expression::ExternalFunctionCallRaw {
                loc: _,
                ty: _,
                address,
                args,
                value,
                gas,
            } => {
                SolangServer::construct_expr(args, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(address, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(value, lookup_tbl, symtab, fnc_map, ns);
                SolangServer::construct_expr(gas, lookup_tbl, symtab, fnc_map, ns);
            }
            Expression::Constructor {
                loc: _,
                contract_no: _,
                constructor_no: _,
                args,
                gas,
                value,
                salt,
                space,
            } => {
                SolangServer::construct_expr(gas, lookup_tbl, symtab, fnc_map, ns);
                for expp in args {
                    SolangServer::construct_expr(expp, lookup_tbl, symtab, fnc_map, ns);
                }
                if let Some(optval) = value {
                    SolangServer::construct_expr(optval, lookup_tbl, symtab, fnc_map, ns);
                }
                if let Some(optsalt) = salt {
                    SolangServer::construct_expr(optsalt, lookup_tbl, symtab, fnc_map, ns);
                }
                if let Some(space) = space {
                    SolangServer::construct_expr(space, lookup_tbl, symtab, fnc_map, ns);
                }
            }

            //Hash table operation expression
            Expression::Keccak256(_locs, _typ, expr) => {
                for expp in expr {
                    SolangServer::construct_expr(expp, lookup_tbl, symtab, fnc_map, ns);
                }
                lookup_tbl.push((_locs.1, _locs.2, String::from("Keccak256 hash")));
            }

            Expression::ReturnData(locs) => {
                let msg = String::from("Return");
                lookup_tbl.push((locs.1, locs.2, msg));
            }
            Expression::Builtin(_locs, _typ, _builtin, expr) => {
                let msg = SolangServer::construct_builtins(_builtin, ns);
                lookup_tbl.push((_locs.1, _locs.2, msg));
                for expp in expr {
                    SolangServer::construct_expr(expp, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            Expression::FormatString(_, sections) => {
                for (_, e) in sections {
                    SolangServer::construct_expr(e, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            Expression::List(_locs, expr) => {
                for expp in expr {
                    SolangServer::construct_expr(expp, lookup_tbl, symtab, fnc_map, ns);
                }
            }
            _ => {}
        }
    }

    // Constructs contract fields and stores it in the lookup table.
    fn construct_cont(
        contvar: &Variable,
        lookup_tbl: &mut Vec<(usize, usize, String)>,
        samptb: &sema::symtable::Symtable,
        fnc_map: &HashMap<String, String>,
        ns: &ast::Namespace,
    ) {
        let msg_typ = SolangServer::expanded_ty(&contvar.ty, ns);
        let msg = format!("{} {}", msg_typ, contvar.name);
        lookup_tbl.push((contvar.loc.1, contvar.loc.2, msg));
        if let Some(expr) = &contvar.initializer {
            SolangServer::construct_expr(expr, lookup_tbl, samptb, fnc_map, ns);
        }
    }

    // Constructs struct fields and stores it in the lookup table.
    fn construct_strct(
        strfld: &Parameter,
        lookup_tbl: &mut Vec<(usize, usize, String)>,
        ns: &ast::Namespace,
    ) {
        let msg_typ = &strfld.ty.to_string(ns);
        let msg = format!("{} {}", msg_typ, strfld.name);
        lookup_tbl.push((strfld.loc.1, strfld.loc.2, msg));
    }

    // Traverses namespace to build messages stored in the lookup table for hover feature.
    fn traverse(
        ns: &ast::Namespace,
        lookup_tbl: &mut Vec<(usize, usize, String)>,
        fnc_map: &mut HashMap<String, String>,
    ) {
        for enm in &ns.enums {
            for (nam, vals) in &enm.values {
                let evnt_msg = format!("{} {}, \n\n", nam, vals.1);
                lookup_tbl.push((vals.0 .1, vals.0 .2, evnt_msg));
            }

            let msg_tg = render(&enm.tags[..]);
            lookup_tbl.push((enm.loc.1, (enm.loc.1 + enm.name.len()), msg_tg));
        }

        for strct in &ns.structs {
            for filds in &strct.fields {
                SolangServer::construct_strct(filds, lookup_tbl, ns);
            }

            let msg_tg = render(&strct.tags[..]);
            lookup_tbl.push((strct.loc.1, (strct.loc.1 + strct.name.len()), msg_tg));
        }

        for fnc in &ns.functions {
            for parm in &fnc.params {
                let msg = SolangServer::expanded_ty(&parm.ty, ns);
                lookup_tbl.push((parm.loc.1, parm.loc.2, msg));
            }

            for ret in &fnc.returns {
                let msg = SolangServer::expanded_ty(&ret.ty, ns);
                lookup_tbl.push((ret.loc.1, ret.loc.2, msg));
            }

            for stmt in &fnc.body {
                SolangServer::construct_stmt(stmt, lookup_tbl, &fnc.symtable, fnc_map, ns);
            }
        }

        for constant in &ns.constants {
            let samptb = symtable::Symtable::new();
            SolangServer::construct_cont(constant, lookup_tbl, &samptb, fnc_map, ns);

            let msg_tg = render(&constant.tags[..]);
            lookup_tbl.push((
                constant.loc.1,
                (constant.loc.1 + constant.name.len()),
                msg_tg,
            ));
        }

        for contrct in &ns.contracts {
            let msg_tg = render(&contrct.tags[..]);
            lookup_tbl.push((contrct.loc.1, (contrct.loc.1 + msg_tg.len()), msg_tg));

            for varscont in &contrct.variables {
                let samptb = symtable::Symtable::new();
                SolangServer::construct_cont(varscont, lookup_tbl, &samptb, fnc_map, ns);

                let msg_tg = render(&varscont.tags[..]);
                lookup_tbl.push((
                    varscont.loc.1,
                    (varscont.loc.1 + varscont.name.len()),
                    msg_tg,
                ));
            }
        }

        for entdcl in &ns.events {
            for filds in &entdcl.fields {
                SolangServer::construct_strct(filds, lookup_tbl, ns);
            }
            let msg_tg = render(&entdcl.tags[..]);
            lookup_tbl.push((entdcl.loc.1, (entdcl.loc.1 + entdcl.name.len()), msg_tg));
        }

        for lookup in lookup_tbl.iter_mut() {
            if let Some(msg) = ns.hover_overrides.get(&pt::Loc(0, lookup.0, lookup.1)) {
                lookup.2 = msg.clone();
            }
        }
    }

    /// Render the type with struct/enum fields expanded
    fn expanded_ty(ty: &sema::ast::Type, ns: &ast::Namespace) -> String {
        match ty {
            sema::ast::Type::Ref(ty) => SolangServer::expanded_ty(ty, ns),
            sema::ast::Type::StorageRef(_, ty) => SolangServer::expanded_ty(ty, ns),
            sema::ast::Type::Struct(n) => {
                let strct = &ns.structs[*n];

                let mut msg = render(&strct.tags);

                msg.push_str(&format!("```\nstruct {} {{\n", strct));

                let mut iter = strct.fields.iter().peekable();
                while let Some(field) = iter.next() {
                    msg.push_str(&format!(
                        "\t{} {}{}\n",
                        field.ty.to_string(ns),
                        field.name,
                        if iter.peek().is_some() { "," } else { "" }
                    ));
                }

                msg.push_str("};\n```\n");

                msg
            }
            sema::ast::Type::Enum(n) => {
                let enm = &ns.enums[*n];

                let mut msg = render(&enm.tags);

                msg.push_str(&format!("```\nenum {} {{\n", enm));

                // display the enum values in-order
                let mut values = Vec::new();
                values.resize(enm.values.len(), "");

                for (name, value) in &enm.values {
                    values[value.1] = name;
                }

                let mut iter = values.iter().peekable();

                while let Some(value) = iter.next() {
                    msg.push_str(&format!(
                        "\t{}{}\n",
                        value,
                        if iter.peek().is_some() { "," } else { "" }
                    ));
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
                    TextDocumentSyncKind::Incremental,
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
            .log_message(MessageType::Info, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        self.client
            .log_message(MessageType::Info, "workspace folders changed!")
            .await;
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        self.client
            .log_message(MessageType::Info, "configuration changed!")
            .await;
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.client
            .log_message(MessageType::Info, "watched files have changed!")
            .await;
    }

    async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
        self.client
            .log_message(MessageType::Info, "command executed!")
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
                let offset =
                    hovers
                        .offsets
                        .get_offset(0, pos.line as usize, pos.character as usize);

                if let Ok(pos) = hovers.lookup.binary_search_by(|entry| {
                    if entry.0 > offset {
                        Ordering::Greater
                    } else if entry.1 < offset {
                        Ordering::Less
                    } else {
                        Ordering::Equal
                    }
                }) {
                    let msg = &hovers.lookup[pos];
                    let loc = pt::Loc(0, msg.0, msg.1);
                    let range = SolangServer::loc_to_range(&loc, &hovers.offsets);

                    return Ok(Some(Hover {
                        contents: HoverContents::Scalar(MarkedString::String(msg.2.to_string())),
                        range: Some(range),
                    }));
                }
            }
        }

        Ok(None)
    }
}
