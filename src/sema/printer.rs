use super::ast::*;
use hex;
use parser::pt;

#[derive(Clone)]
enum Tree {
    Leaf(String),
    Branch(String, Vec<Tree>),
}

fn print_tree(t: &Tree, prefix: &str, field_prefix: &str) -> String {
    let mut res = String::new();

    match t {
        Tree::Leaf(s) => {
            res.push_str(&format!("{}{}\n", prefix, s));
        }
        Tree::Branch(s, list) => {
            res.push_str(&format!("{}{}\n", prefix, s));

            let len = list.len();

            for (i, e) in list.iter().enumerate() {
                if len != i + 1 {
                    res.push_str(&format!(
                        "{}├─ {}",
                        field_prefix,
                        print_tree(e, prefix, &format!("{}│  ", field_prefix))
                    ));
                } else {
                    res.push_str(&format!(
                        "{}└─ {}",
                        field_prefix,
                        print_tree(e, prefix, &format!("{}   ", field_prefix))
                    ));
                }
            }
        }
    }

    res
}

fn print_expr(e: &Expression, func: Option<&Function>, ns: &Namespace) -> Tree {
    match e {
        Expression::BoolLiteral(_, b) => Tree::Leaf(format!(
            "literal bool {}",
            if *b { "true" } else { "false" }
        )),
        Expression::BytesLiteral(_, ty, b) => {
            Tree::Leaf(format!("literal {} {}", ty.to_string(ns), hex::encode(b)))
        }
        Expression::CodeLiteral(_, contract_no, true) => {
            Tree::Leaf(format!("code runtime {}", ns.contracts[*contract_no].name))
        }
        Expression::CodeLiteral(_, contract_no, false) => {
            Tree::Leaf(format!("code deploy {}", ns.contracts[*contract_no].name))
        }
        Expression::NumberLiteral(_, ty, b) => {
            Tree::Leaf(format!("literal {} {}", ty.to_string(ns), b))
        }
        Expression::StructLiteral(_, ty, fields) => {
            let fields = fields.iter().map(|e| print_expr(e, func, ns)).collect();

            Tree::Branch(format!("struct {}", ty.to_string(ns)), fields)
        }
        Expression::Add(_, ty, left, right) => Tree::Branch(
            format!("add {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::Subtract(_, ty, left, right) => Tree::Branch(
            format!("subtract {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::Multiply(_, ty, left, right) => Tree::Branch(
            format!("multiply {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::UDivide(_, ty, left, right) => Tree::Branch(
            format!("unsigned divide {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::SDivide(_, ty, left, right) => Tree::Branch(
            format!("signed divide {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::UModulo(_, ty, left, right) => Tree::Branch(
            format!("unsigned modulo {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::SModulo(_, ty, left, right) => Tree::Branch(
            format!("signed modulo {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::Power(_, ty, left, right) => Tree::Branch(
            format!("power {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::BitwiseOr(_, ty, left, right) => Tree::Branch(
            format!("bitwise or {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::BitwiseAnd(_, ty, left, right) => Tree::Branch(
            format!("bitwise and {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::BitwiseXor(_, ty, left, right) => Tree::Branch(
            format!("bitwise xor {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::ShiftLeft(_, ty, left, right) => Tree::Branch(
            format!("shift left {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::ShiftRight(_, ty, left, right, sign) => Tree::Branch(
            format!("shift right {}", ty.to_string(ns)),
            vec![
                print_expr(left, func, ns),
                print_expr(right, func, ns),
                Tree::Leaf(format!("signed: {}", *sign)),
            ],
        ),
        Expression::Variable(_, ty, pos) => Tree::Leaf(format!(
            "variable {} {}",
            ty.to_string(ns),
            func.unwrap().symtable.vars[pos].id.name
        )),
        Expression::ConstantVariable(_, ty, base_contract_no, var_no) => Tree::Leaf(format!(
            "contract variable {} {}",
            ty.to_string(ns),
            ns.contracts[*base_contract_no].variables[*var_no].name
        )),
        Expression::StorageVariable(_, ty, base_contract_no, var_no) => Tree::Leaf(format!(
            "storage variable {} {}",
            ty.to_string(ns),
            ns.contracts[*base_contract_no].variables[*var_no].name
        )),
        Expression::Load(_, ty, expr) => Tree::Branch(
            format!("load memory {}", ty.to_string(ns)),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::StorageLoad(_, ty, expr) => Tree::Branch(
            format!("load storage {}", ty.to_string(ns)),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::ZeroExt(_, ty, expr) => Tree::Branch(
            format!("zero extend {}", ty.to_string(ns)),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::SignExt(_, ty, expr) => Tree::Branch(
            format!("sign extend {}", ty.to_string(ns)),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::Trunc(_, ty, expr) => Tree::Branch(
            format!("truncate {}", ty.to_string(ns)),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::Cast(_, ty, expr) => Tree::Branch(
            format!("cast {}", ty.to_string(ns)),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::PreIncrement(_, ty, expr) => Tree::Branch(
            format!("pre-increment {}", ty.to_string(ns)),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::PreDecrement(_, ty, expr) => Tree::Branch(
            format!("pre-decrement {}", ty.to_string(ns)),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::PostIncrement(_, ty, expr) => Tree::Branch(
            format!("post-increment {}", ty.to_string(ns)),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::PostDecrement(_, ty, expr) => Tree::Branch(
            format!("post-decrement {}", ty.to_string(ns)),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::Assign(_, ty, left, right) => Tree::Branch(
            format!("assign {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::More(_, left, right) => Tree::Branch(
            String::from("more"),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::Less(_, left, right) => Tree::Branch(
            String::from("less"),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::MoreEqual(_, left, right) => Tree::Branch(
            String::from("more or equal"),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::LessEqual(_, left, right) => Tree::Branch(
            String::from("less or equal"),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::Equal(_, left, right) => Tree::Branch(
            String::from("equal"),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::NotEqual(_, left, right) => Tree::Branch(
            String::from("not equal"),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::Not(_, expr) => {
            Tree::Branch(String::from("not"), vec![print_expr(expr, func, ns)])
        }
        Expression::Complement(_, ty, expr) => Tree::Branch(
            format!("complement {}", ty.to_string(ns)),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::UnaryMinus(_, ty, expr) => Tree::Branch(
            format!("unary minus {}", ty.to_string(ns)),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::Ternary(_, _, cond, left, right) => Tree::Branch(
            String::from("ternary"),
            vec![
                print_expr(cond, func, ns),
                print_expr(left, func, ns),
                print_expr(right, func, ns),
            ],
        ),
        Expression::ArraySubscript(_, ty, array, index) => Tree::Branch(
            format!("array subscript {}", ty.to_string(ns)),
            vec![
                Tree::Branch(String::from("array"), vec![print_expr(array, func, ns)]),
                Tree::Branch(String::from("index"), vec![print_expr(index, func, ns)]),
            ],
        ),
        Expression::StructMember(_, ty, struct_expr, member) => {
            if let Type::Struct(struct_no) = struct_expr.ty() {
                Tree::Branch(
                    format!("array subscript {}", ty.to_string(ns)),
                    vec![
                        Tree::Branch(
                            String::from("struct"),
                            vec![print_expr(struct_expr, func, ns)],
                        ),
                        Tree::Branch(
                            String::from("member"),
                            vec![Tree::Leaf(
                                ns.structs[struct_no].fields[*member].name.to_owned(),
                            )],
                        ),
                    ],
                )
            } else {
                panic!("struct member on non-struct");
            }
        }
        Expression::Or(_, left, right) => Tree::Branch(
            String::from("logical or"),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::And(_, left, right) => Tree::Branch(
            String::from("logical and"),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::InternalFunction {
            contract_no,
            function_no,
            signature,
            ..
        } => Tree::Leaf(format!(
            "function {}.{} {:?}",
            ns.contracts[*contract_no].name,
            ns.contracts[*contract_no].functions[*function_no].name,
            signature,
        )),
        Expression::ExternalFunction {
            contract_no,
            function_no,
            ..
        } => Tree::Leaf(format!(
            "function external {}.{}",
            ns.contracts[*contract_no].name,
            ns.contracts[*contract_no].functions[*function_no].name,
        )),
        _ => Tree::Leaf(String::from("not implemented")),
    }
}

fn print_statement(stmts: &[Statement], func: &Function, ns: &Namespace) -> Vec<Tree> {
    let mut res = Vec::new();

    for stmt in stmts {
        res.push(match stmt {
            Statement::VariableDecl(_, _, p, None) => {
                Tree::Leaf(format!("declare {} {}", p.ty.to_string(ns), p.name))
            }
            Statement::VariableDecl(_, _, p, Some(e)) => Tree::Branch(
                format!("declare {} {}", p.ty.to_string(ns), p.name),
                vec![print_expr(e, Some(func), ns)],
            ),
            Statement::If(_, _, cond, then_stmt, else_stmt) if else_stmt.is_empty() => {
                let cond =
                    Tree::Branch(String::from("cond"), vec![print_expr(cond, Some(func), ns)]);
                let then = Tree::Branch(String::from("then"), print_statement(then_stmt, func, ns));
                Tree::Branch(String::from("if"), vec![cond, then])
            }
            Statement::If(_, _, cond, then_stmt, else_stmt) => {
                let cond =
                    Tree::Branch(String::from("cond"), vec![print_expr(cond, Some(func), ns)]);
                let then_tree =
                    Tree::Branch(String::from("then"), print_statement(then_stmt, func, ns));
                let else_tree =
                    Tree::Branch(String::from("else"), print_statement(else_stmt, func, ns));
                Tree::Branch(String::from("if"), vec![cond, then_tree, else_tree])
            }
            Statement::While(_, _, cond, body) => {
                let cond =
                    Tree::Branch(String::from("cond"), vec![print_expr(cond, Some(func), ns)]);
                let body = Tree::Branch(String::from("then"), print_statement(body, func, ns));
                Tree::Branch(String::from("while"), vec![cond, body])
            }
            Statement::For {
                init,
                cond: None,
                next,
                body,
                ..
            } => {
                let init = Tree::Branch(String::from("init"), print_statement(init, func, ns));
                let body = Tree::Branch(String::from("body"), print_statement(body, func, ns));
                let next = Tree::Branch(String::from("next"), print_statement(next, func, ns));
                Tree::Branch(String::from("for"), vec![init, body, next])
            }
            Statement::For {
                init,
                cond: Some(cond),
                next,
                body,
                ..
            } => {
                let init = Tree::Branch(String::from("init"), print_statement(init, func, ns));
                let cond =
                    Tree::Branch(String::from("cond"), vec![print_expr(cond, Some(func), ns)]);
                let body = Tree::Branch(String::from("body"), print_statement(body, func, ns));
                let next = Tree::Branch(String::from("next"), print_statement(next, func, ns));
                Tree::Branch(String::from("for"), vec![init, cond, body, next])
            }
            Statement::DoWhile(_, _, body, cond) => {
                let body = Tree::Branch(String::from("then"), print_statement(body, func, ns));
                let cond =
                    Tree::Branch(String::from("cond"), vec![print_expr(cond, Some(func), ns)]);
                Tree::Branch(String::from("do while"), vec![body, cond])
            }
            Statement::Expression(_, _, expr) => Tree::Branch(
                String::from("expression"),
                vec![print_expr(expr, Some(func), ns)],
            ),
            Statement::Delete(_, ty, expr) => {
                let expr =
                    Tree::Branch(String::from("expr"), vec![print_expr(expr, Some(func), ns)]);
                let ty = Tree::Leaf(ty.to_string(ns));
                Tree::Branch(String::from("delete"), vec![expr, ty])
            }
            Statement::Break(_) => Tree::Leaf(String::from("break")),
            Statement::Continue(_) => Tree::Leaf(String::from("continue")),
            Statement::Return(_, args) => {
                if args.is_empty() {
                    Tree::Leaf(String::from("return"))
                } else {
                    let args = args.iter().map(|e| print_expr(e, Some(func), ns)).collect();

                    Tree::Branch(String::from("return"), args)
                }
            }
            Statement::Emit { event_no, args, .. } => {
                let args = args.iter().map(|e| print_expr(e, Some(func), ns)).collect();

                Tree::Branch(format!("emit {}", ns.events[*event_no].to_string()), args)
            }
            Statement::Destructure(_, fields, args) => {
                let fields = fields
                    .iter()
                    .map(|f| match f {
                        DestructureField::None => Tree::Leaf(String::from("")),
                        DestructureField::Expression(e) => print_expr(e, Some(func), ns),
                        DestructureField::VariableDecl(_, p) => {
                            Tree::Leaf(format!("{} {}", p.ty.to_string(ns), p.name))
                        }
                    })
                    .collect();
                let args = print_expr(args, Some(func), ns);

                Tree::Branch(
                    String::from("destructure"),
                    vec![Tree::Branch(String::from("fields"), fields), args],
                )
            }
            Statement::TryCatch {
                expr,
                returns,
                ok_stmt,
                error,
                catch_param,
                catch_stmt,
                ..
            } => {
                let mut list = Vec::new();

                list.push(Tree::Branch(
                    String::from("expr"),
                    vec![print_expr(expr, Some(func), ns)],
                ));

                if !returns.is_empty() {
                    let returns = returns
                        .iter()
                        .map(|(_, param)| {
                            Tree::Leaf(format!("{} {}", param.ty.to_string(ns), param.name))
                        })
                        .collect();

                    list.push(Tree::Branch(String::from("returns"), returns));
                }

                list.push(Tree::Branch(
                    String::from("ok_stmt"),
                    print_statement(ok_stmt, func, ns),
                ));

                if let Some((_, param, stmt)) = &error {
                    list.push(Tree::Leaf(format!(
                        "error_param: {} {}",
                        param.ty.to_string(ns),
                        param.name
                    )));

                    list.push(Tree::Branch(
                        String::from("error_statement"),
                        print_statement(stmt, func, ns),
                    ));
                }

                list.push(Tree::Leaf(format!(
                    "catch_param: {} {}",
                    catch_param.ty.to_string(ns),
                    catch_param.name
                )));

                list.push(Tree::Branch(
                    String::from("catch_stmt"),
                    print_statement(catch_stmt, func, ns),
                ));

                Tree::Branch(String::from("try-catch"), list)
            }
            Statement::Underscore(_) => Tree::Leaf(String::from("underscore")),
        });
    }

    res
}

impl Namespace {
    pub fn print(&self, filename: &str) -> String {
        // enums
        let mut t = Vec::new();
        for e in &self.enums {
            let mut values = Vec::new();
            values.resize(e.values.len(), Tree::Leaf(String::new()));
            for (name, (_, pos)) in &e.values {
                values[*pos] = Tree::Leaf(name.clone());
            }

            t.push(Tree::Branch(format!("enum {}", e), values));
        }

        // structs
        for s in &self.structs {
            let fields = s
                .fields
                .iter()
                .map(|p| Tree::Leaf(format!("field {} {}", p.ty.to_string(&self), p.name)))
                .collect();

            t.push(Tree::Branch(format!("struct {}", s), fields));
        }

        // events
        for e in &self.events {
            let fields = e
                .fields
                .iter()
                .map(|p| {
                    Tree::Leaf(format!(
                        "field {} {}{}",
                        p.ty.to_string(&self),
                        if p.indexed { "indexed " } else { "" },
                        p.name
                    ))
                })
                .collect();

            t.push(Tree::Branch(
                format!("event {} {}", e, if e.anonymous { "anonymous" } else { "" }),
                fields,
            ));
        }

        // contracts
        for c in &self.contracts {
            let mut members = Vec::new();

            if !c.bases.is_empty() {
                let mut list = Vec::new();

                for base in &c.bases {
                    let name = self.contracts[base.contract_no].name.clone();

                    if let Some((_, args)) = &base.constructor {
                        list.push(Tree::Branch(
                            name,
                            args.iter().map(|e| print_expr(e, None, self)).collect(),
                        ));
                    } else {
                        list.push(Tree::Leaf(name));
                    }
                }

                members.push(Tree::Branch(String::from("bases"), list));
            }

            for var in &c.variables {
                let name = format!(
                    "variable {} {} {}",
                    if var.constant { "constant" } else { "storage" },
                    var.ty.to_string(self),
                    var.name
                );

                if let Some(initializer) = &var.initializer {
                    members.push(Tree::Branch(
                        name,
                        vec![print_expr(initializer, None, self)],
                    ));
                } else {
                    members.push(Tree::Leaf(name));
                }
            }

            if !c.using.is_empty() {
                let mut list = Vec::new();

                for (library_no, ty) in &c.using {
                    if let Some(ty) = ty {
                        list.push(Tree::Leaf(format!(
                            "library {} for {}",
                            self.contracts[*library_no].name,
                            ty.to_string(self)
                        )));
                    } else {
                        list.push(Tree::Leaf(format!(
                            "library {}",
                            self.contracts[*library_no].name
                        )));
                    }
                }

                members.push(Tree::Branch(String::from("using"), list));
            }

            for func in &c.functions {
                let mut list = Vec::new();

                list.push(Tree::Leaf(format!("visibility {}", func.visibility)));

                if func.ty == pt::FunctionTy::Constructor && func.ty == pt::FunctionTy::Function {
                    list.push(Tree::Leaf(format!("signature {}", func.signature)));
                }

                if let Some(mutability) = &func.mutability {
                    list.push(Tree::Leaf(format!("mutability {}", mutability)));
                }

                if func.is_virtual {
                    list.push(Tree::Leaf(String::from("virtual")));
                }

                if let Some((_, is_override)) = &func.is_override {
                    if is_override.is_empty() {
                        list.push(Tree::Leaf(String::from("override")));
                    } else {
                        list.push(Tree::Branch(
                            String::from("override"),
                            is_override
                                .iter()
                                .map(|contract_no| {
                                    Tree::Leaf(self.contracts[*contract_no].name.clone())
                                })
                                .collect(),
                        ));
                    }
                }

                if !func.bases.is_empty() {
                    let mut list = Vec::new();
                    for (base_no, (_, _, args)) in &func.bases {
                        let name = self.contracts[*base_no].name.clone();
                        if !args.is_empty() {
                            list.push(Tree::Branch(
                                name,
                                args.iter()
                                    .map(|e| print_expr(e, Some(func), self))
                                    .collect(),
                            ));
                        } else {
                            list.push(Tree::Leaf(name));
                        }
                    }
                    members.push(Tree::Branch(String::from("bases"), list));
                }

                if !func.params.is_empty() {
                    let params = func
                        .params
                        .iter()
                        .map(|p| Tree::Leaf(format!("{} {}", p.ty.to_string(&self), p.name)))
                        .collect();

                    list.push(Tree::Branch(String::from("params"), params));
                }

                if !func.returns.is_empty() {
                    let returns = func
                        .returns
                        .iter()
                        .map(|p| Tree::Leaf(format!("{} {}", p.ty.to_string(&self), p.name)))
                        .collect();

                    list.push(Tree::Branch(String::from("returns"), returns));
                }

                if !func.body.is_empty() {
                    list.push(Tree::Branch(
                        String::from("body"),
                        print_statement(&func.body, func, self),
                    ));
                }

                members.push(Tree::Branch(format!("{} {}", func.ty, func.name), list));
            }

            t.push(Tree::Branch(format!("{} {}", c.ty, c.name), members));
        }

        print_tree(&Tree::Branch(filename.to_owned(), t), "", "")
    }
}
