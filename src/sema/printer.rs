use super::ast::*;
use super::builtin::{get_prototype, Prototype};
use crate::parser::pt;
use crate::Target;
use num_traits::ToPrimitive;

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
        Expression::BytesLiteral(_, Type::String, bs) => Tree::Leaf(format!(
            "literal string \"{}\"",
            String::from_utf8_lossy(bs)
        )),
        Expression::BytesLiteral(_, ty, b) => {
            Tree::Leaf(format!("literal {} {}", ty.to_string(ns), hex::encode(b)))
        }
        Expression::ArrayLiteral(_, ty, _, values) => Tree::Branch(
            format!("array literal {}", ty.to_string(ns)),
            values.iter().map(|e| print_expr(e, func, ns)).collect(),
        ),
        Expression::ConstArrayLiteral(_, ty, _, values) => Tree::Branch(
            format!("const array literal {}", ty.to_string(ns)),
            values.iter().map(|e| print_expr(e, func, ns)).collect(),
        ),
        Expression::CodeLiteral(_, contract_no, true) => {
            Tree::Leaf(format!("code runtime {}", ns.contracts[*contract_no].name))
        }
        Expression::CodeLiteral(_, contract_no, false) => {
            Tree::Leaf(format!("code deploy {}", ns.contracts[*contract_no].name))
        }
        // TODO does not format negative constants correctly
        Expression::NumberLiteral(_, ty @ Type::Address(_), b) => Tree::Leaf(format!(
            "literal {} {:#02$x}",
            ty.to_string(ns),
            b,
            ns.address_length * 2 + 2,
        )),
        Expression::NumberLiteral(_, ty, b) => {
            Tree::Leaf(format!("literal {} {}", ty.to_string(ns), b))
        }
        Expression::RationalNumberLiteral(_, ty, b) => Tree::Leaf(format!(
            "literal {} {}",
            ty.to_string(ns),
            b.to_f64().unwrap()
        )),
        Expression::StructLiteral(_, ty, fields) => {
            let fields = fields.iter().map(|e| print_expr(e, func, ns)).collect();

            Tree::Branch(format!("struct {}", ty.to_string(ns)), fields)
        }
        Expression::Add(_, ty, unchecked, left, right) => Tree::Branch(
            format!(
                "add {}{}",
                if *unchecked { "unchecked " } else { "" },
                ty.to_string(ns)
            ),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::Subtract(_, ty, unchecked, left, right) => Tree::Branch(
            format!(
                "subtract {}{}",
                if *unchecked { "unchecked " } else { "" },
                ty.to_string(ns)
            ),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::Multiply(_, ty, unchecked, left, right) => Tree::Branch(
            format!(
                "multiply {}{}",
                if *unchecked { "unchecked " } else { "" },
                ty.to_string(ns)
            ),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::Divide(_, ty, left, right) => Tree::Branch(
            format!("divide {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::Modulo(_, ty, left, right) => Tree::Branch(
            format!("modulo {}", ty.to_string(ns)),
            vec![print_expr(left, func, ns), print_expr(right, func, ns)],
        ),
        Expression::Power(_, ty, unchecked, left, right) => Tree::Branch(
            format!(
                "power {}{}",
                if *unchecked { "unchecked " } else { "" },
                ty.to_string(ns)
            ),
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
        Expression::ConstantVariable(_, ty, Some(base_contract_no), var_no) => Tree::Leaf(format!(
            "contract variable {} {}",
            ty.to_string(ns),
            ns.contracts[*base_contract_no].variables[*var_no].name
        )),
        Expression::ConstantVariable(_, ty, None, var_no) => Tree::Leaf(format!(
            "contract variable {} {}",
            ty.to_string(ns),
            ns.constants[*var_no].name
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
        Expression::BytesCast(_, ty, from, expr) => Tree::Branch(
            format!(
                "bytes cast to {} from {}",
                ty.to_string(ns),
                from.to_string(ns)
            ),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::PreIncrement(_, ty, unchecked, expr) => Tree::Branch(
            format!(
                "pre-increment {}{}",
                if *unchecked { "unchecked " } else { "" },
                ty.to_string(ns)
            ),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::PreDecrement(_, ty, unchecked, expr) => Tree::Branch(
            format!(
                "pre-decrement {}{}",
                if *unchecked { "unchecked " } else { "" },
                ty.to_string(ns)
            ),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::PostIncrement(_, ty, unchecked, expr) => Tree::Branch(
            format!(
                "post-increment {}{}",
                if *unchecked { "unchecked " } else { "" },
                ty.to_string(ns)
            ),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::PostDecrement(_, ty, unchecked, expr) => Tree::Branch(
            format!(
                "post-decrement {}{}",
                if *unchecked { "unchecked " } else { "" },
                ty.to_string(ns)
            ),
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
        Expression::Subscript(_, _, ty, array, index) => Tree::Branch(
            format!("subscript {}", ty.to_string(ns)),
            vec![
                Tree::Branch(String::from("array"), vec![print_expr(array, func, ns)]),
                Tree::Branch(String::from("index"), vec![print_expr(index, func, ns)]),
            ],
        ),
        Expression::StructMember(_, ty, struct_expr, member) => {
            if let Type::Struct(struct_no) = struct_expr.ty().deref_any() {
                Tree::Branch(
                    format!("struct member {}", ty.to_string(ns)),
                    vec![
                        Tree::Branch(
                            String::from("struct"),
                            vec![print_expr(struct_expr, func, ns)],
                        ),
                        Tree::Branch(
                            String::from("field"),
                            vec![Tree::Leaf(
                                ns.structs[*struct_no].fields[*member].name.to_owned(),
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
            function_no,
            signature,
            ..
        } => Tree::Leaf(format!(
            "function {} {:?}",
            ns.functions[*function_no].print_name(ns),
            signature,
        )),
        Expression::ExternalFunction { function_no, .. } => Tree::Leaf(format!(
            "external {}",
            ns.functions[*function_no].print_name(ns)
        )),
        Expression::Builtin(_, _, builtin, args) => {
            let title = match get_prototype(*builtin) {
                Some(Prototype {
                    namespace: None,
                    name,
                    ..
                }) => format!("builtin {}", name),
                Some(Prototype {
                    namespace: Some(namespace),
                    name,
                    ..
                }) => format!("builtin {}.{}", namespace, name),
                _ => String::from("unknown builtin"),
            };
            Tree::Branch(
                title,
                args.iter().map(|e| print_expr(e, func, ns)).collect(),
            )
        }
        Expression::StringCompare(_, left, right) => Tree::Branch(
            String::from("string compare"),
            vec![
                print_string_location(left, func, ns),
                print_string_location(right, func, ns),
            ],
        ),
        Expression::StringConcat(_, _, left, right) => Tree::Branch(
            String::from("string concatenate"),
            vec![
                print_string_location(left, func, ns),
                print_string_location(right, func, ns),
            ],
        ),
        Expression::InterfaceId(_, contract_no) => {
            Tree::Leaf(format!("interfaceId {}", ns.contracts[*contract_no].name))
        }
        Expression::FormatString(_, args) => Tree::Branch(
            String::from("format"),
            args.iter()
                .map(|(_, arg)| print_expr(arg, func, ns))
                .collect(),
        ),
        Expression::StorageArrayLength { array, .. } => Tree::Branch(
            String::from("storage array length"),
            vec![print_expr(array, func, ns)],
        ),
        Expression::InternalFunctionCall { function, args, .. } => Tree::Branch(
            String::from("internal function call"),
            vec![
                print_expr(function, func, ns),
                Tree::Branch(
                    String::from("args"),
                    args.iter().map(|e| print_expr(e, func, ns)).collect(),
                ),
            ],
        ),
        Expression::ExternalFunctionCall { function, args, .. } => Tree::Branch(
            String::from("external function call"),
            vec![
                print_expr(function, func, ns),
                Tree::Branch(
                    String::from("args"),
                    args.iter().map(|e| print_expr(e, func, ns)).collect(),
                ),
            ],
        ),
        Expression::ExternalFunctionCallRaw { address, args, .. } => Tree::Branch(
            String::from("external function call raw"),
            vec![print_expr(address, func, ns), print_expr(args, func, ns)],
        ),
        Expression::Constructor {
            contract_no,
            value,
            salt,
            space,
            args,
            ..
        } => {
            let mut list = vec![Tree::Leaf(format!(
                "contract {}",
                ns.contracts[*contract_no].name
            ))];

            if let Some(value) = value {
                list.push(Tree::Branch(
                    String::from("value"),
                    vec![print_expr(value, func, ns)],
                ));
            }

            if let Some(salt) = salt {
                list.push(Tree::Branch(
                    String::from("salt"),
                    vec![print_expr(salt, func, ns)],
                ));
            }

            if let Some(space) = space {
                list.push(Tree::Branch(
                    String::from("space"),
                    vec![print_expr(space, func, ns)],
                ));
            }

            list.push(Tree::Branch(
                String::from("args"),
                args.iter().map(|e| print_expr(e, func, ns)).collect(),
            ));

            Tree::Branch(String::from("constructor"), list)
        }
        Expression::AllocDynamicArray(_, ty, expr, None) => Tree::Branch(
            format!("allocate dynamic array {}", ty.to_string(ns)),
            vec![print_expr(expr, func, ns)],
        ),
        Expression::AllocDynamicArray(_, ty, expr, Some(init)) => Tree::Branch(
            format!("allocate dynamic array {}", ty.to_string(ns)),
            vec![
                print_expr(expr, func, ns),
                Tree::Leaf(format!("init {}", hex::encode(init))),
            ],
        ),
        Expression::StorageBytesSubscript(_, array, index) => Tree::Branch(
            String::from("storage bytes subscript"),
            vec![
                Tree::Branch(String::from("array"), vec![print_expr(array, func, ns)]),
                Tree::Branch(String::from("index"), vec![print_expr(index, func, ns)]),
            ],
        ),
        Expression::List(_, list) => Tree::Branch(
            String::from("list"),
            list.iter().map(|e| print_expr(e, func, ns)).collect(),
        ),
        Expression::FunctionArg(_, ty, no) => {
            Tree::Leaf(format!("func arg #{} {}", no, ty.to_string(ns)))
        }
        Expression::InternalFunctionCfg(..)
        | Expression::ReturnData(..)
        | Expression::Poison
        | Expression::Undefined(_)
        | Expression::AbiEncode { .. }
        | Expression::Keccak256(..) => {
            panic!("should not present in ast");
        }
    }
}

fn print_string_location(s: &StringLocation, func: Option<&Function>, ns: &Namespace) -> Tree {
    match s {
        StringLocation::CompileTime(val) => Tree::Leaf(format!("hex\"{}\"", hex::encode(val))),
        StringLocation::RunTime(expr) => print_expr(expr, func, ns),
    }
}

fn print_statement(stmts: &[Statement], func: &Function, ns: &Namespace) -> Vec<Tree> {
    let mut res = Vec::new();

    for stmt in stmts {
        res.push(match stmt {
            Statement::Block {
                statements,
                unchecked,
                ..
            } => Tree::Branch(
                format!("block{}", if *unchecked { " unchecked" } else { "" }),
                print_statement(statements, func, ns),
            ),
            Statement::VariableDecl(_, _, p, None) => {
                Tree::Leaf(format!("declare {} {}", p.ty.to_string(ns), p.name))
            }
            Statement::VariableDecl(_, _, p, Some(e)) => Tree::Branch(
                format!("declare {} {}", p.ty.to_string(ns), p.name),
                vec![print_expr(e, Some(func), ns)],
            ),
            Statement::If(_, reachable, cond, then_stmt, else_stmt) if else_stmt.is_empty() => {
                let reachable = Tree::Leaf(format!("reachable:{}", reachable));
                let cond =
                    Tree::Branch(String::from("cond"), vec![print_expr(cond, Some(func), ns)]);
                let then = Tree::Branch(String::from("then"), print_statement(then_stmt, func, ns));
                Tree::Branch(String::from("if"), vec![cond, then, reachable])
            }
            Statement::If(_, reachable, cond, then_stmt, else_stmt) => {
                let reachable = Tree::Leaf(format!("reachable:{}", reachable));
                let cond =
                    Tree::Branch(String::from("cond"), vec![print_expr(cond, Some(func), ns)]);
                let then_tree =
                    Tree::Branch(String::from("then"), print_statement(then_stmt, func, ns));
                let else_tree =
                    Tree::Branch(String::from("else"), print_statement(else_stmt, func, ns));
                Tree::Branch(
                    String::from("if"),
                    vec![cond, then_tree, else_tree, reachable],
                )
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
            Statement::Return(_, None) => Tree::Leaf(String::from("return")),
            Statement::Return(_, Some(expr)) => Tree::Branch(
                String::from("return"),
                vec![print_expr(expr, Some(func), ns)],
            ),
            Statement::Emit { event_no, args, .. } => {
                let args = args.iter().map(|e| print_expr(e, Some(func), ns)).collect();

                Tree::Branch(
                    format!("emit {}", ns.events[*event_no].symbol_name(ns)),
                    args,
                )
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
            Statement::TryCatch(_, _, try_catch) => {
                let mut list = vec![Tree::Branch(
                    String::from("expr"),
                    vec![print_expr(&try_catch.expr, Some(func), ns)],
                )];

                if !try_catch.returns.is_empty() {
                    let returns = try_catch
                        .returns
                        .iter()
                        .map(|(_, param)| {
                            Tree::Leaf(format!("{} {}", param.ty.to_string(ns), param.name))
                        })
                        .collect();

                    list.push(Tree::Branch(String::from("returns"), returns));
                }

                list.push(Tree::Branch(
                    String::from("ok_stmt"),
                    print_statement(&try_catch.ok_stmt, func, ns),
                ));

                if let Some((_, param, stmt)) = &try_catch.error {
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

                if let Some(param) = &try_catch.catch_param {
                    list.push(Tree::Leaf(format!(
                        "catch_param: {} {}",
                        param.ty.to_string(ns),
                        param.name
                    )));
                }

                list.push(Tree::Branch(
                    String::from("catch_stmt"),
                    print_statement(&try_catch.catch_stmt, func, ns),
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
                .map(|p| Tree::Leaf(format!("field {} {}", p.ty.to_string(self), p.name)))
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
                        p.ty.to_string(self),
                        if p.indexed { "indexed " } else { "" },
                        p.name
                    ))
                })
                .collect();

            t.push(Tree::Branch(
                format!(
                    "event {} {}",
                    e.symbol_name(self),
                    if e.anonymous { "anonymous" } else { "" }
                ),
                fields,
            ));
        }

        // functions
        for func in &self.functions {
            if func.contract_no.is_none() {
                t.push(print_func(func, self));
            }
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

            for function_no in &c.functions {
                let func = &self.functions[*function_no];

                members.push(print_func(func, self));
            }

            t.push(Tree::Branch(format!("{} {}", c.ty, c.name), members));
        }

        print_tree(&Tree::Branch(filename.to_owned(), t), "", "")
    }

    /// Type storage
    pub fn storage_type(&self) -> Type {
        if self.target == Target::Solana {
            Type::Uint(32)
        } else {
            Type::Uint(256)
        }
    }
}

fn print_func(func: &Function, ns: &Namespace) -> Tree {
    let mut list = vec![Tree::Leaf(format!("visibility {}", func.visibility))];

    if func.ty == pt::FunctionTy::Constructor && func.ty == pt::FunctionTy::Function {
        list.push(Tree::Leaf(format!("signature {}", func.signature)));
    }

    list.push(Tree::Leaf(format!("mutability {}", func.mutability)));

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
                    .map(|contract_no| Tree::Leaf(ns.contracts[*contract_no].name.clone()))
                    .collect(),
            ));
        }
    }

    if !func.bases.is_empty() {
        let mut bases = Vec::new();
        for (base_no, (_, _, args)) in &func.bases {
            let name = ns.contracts[*base_no].name.clone();
            if !args.is_empty() {
                bases.push(Tree::Branch(
                    name,
                    args.iter().map(|e| print_expr(e, Some(func), ns)).collect(),
                ));
            } else {
                bases.push(Tree::Leaf(name));
            }
        }
        list.push(Tree::Branch(String::from("bases"), bases));
    }

    if !func.params.is_empty() {
        let params = func
            .params
            .iter()
            .map(|p| Tree::Leaf(format!("{} {}", p.ty.to_string(ns), p.name)))
            .collect();

        list.push(Tree::Branch(String::from("params"), params));
    }

    if !func.returns.is_empty() {
        let returns = func
            .returns
            .iter()
            .map(|p| Tree::Leaf(format!("{} {}", p.ty.to_string(ns), p.name)))
            .collect();

        list.push(Tree::Branch(String::from("returns"), returns));
    }

    if !func.body.is_empty() {
        list.push(Tree::Branch(
            String::from("body"),
            print_statement(&func.body, func, ns),
        ));
    }

    Tree::Branch(format!("{} {}", func.ty, func.name), list)
}
