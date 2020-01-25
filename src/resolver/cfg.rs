use num_bigint::BigInt;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::LinkedList;

use hex;
use output;
use output::Output;
use parser::ast;
use resolver;
use resolver::expression::{cast, expression, Expression};

pub enum Instr {
    FuncArg {
        res: usize,
        arg: usize,
    },
    GetStorage {
        local: usize,
        storage: usize,
    },
    SetStorage {
        local: usize,
        storage: usize,
    },
    Set {
        res: usize,
        expr: Expression,
    },
    Constant {
        res: usize,
        constant: usize,
    },
    Call {
        res: Vec<usize>,
        func: usize,
        args: Vec<Expression>,
    },
    Return {
        value: Vec<Expression>,
    },
    Branch {
        bb: usize,
    },
    BranchCond {
        cond: Expression,
        true_: usize,
        false_: usize,
    },
    AssertFailure {},
}

pub struct BasicBlock {
    pub phis: Option<HashSet<usize>>,
    pub name: String,
    pub instr: Vec<Instr>,
}

impl BasicBlock {
    fn add(&mut self, ins: Instr) {
        self.instr.push(ins);
    }
}

#[derive(Default)]
pub struct ControlFlowGraph {
    pub vars: Vec<Variable>,
    pub bb: Vec<BasicBlock>,
    current: usize,
    pub writes_contract_storage: bool,
    pub reads_contract_storage: bool,
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        let mut cfg = ControlFlowGraph {
            vars: Vec::new(),
            bb: Vec::new(),
            current: 0,
            reads_contract_storage: false,
            writes_contract_storage: false,
        };

        cfg.new_basic_block("entry".to_string());

        cfg
    }

    pub fn new_basic_block(&mut self, name: String) -> usize {
        let pos = self.bb.len();

        self.bb.push(BasicBlock {
            name,
            instr: Vec::new(),
            phis: None,
        });

        pos
    }

    pub fn set_phis(&mut self, bb: usize, phis: HashSet<usize>) {
        if !phis.is_empty() {
            self.bb[bb].phis = Some(phis);
        }
    }

    pub fn set_basic_block(&mut self, pos: usize) {
        self.current = pos;
    }

    pub fn add(&mut self, vartab: &mut Vartable, ins: Instr) {
        if let Instr::Set { res, .. } = ins {
            vartab.set_dirty(res);
        }
        self.bb[self.current].add(ins);
    }

    pub fn expr_to_string(&self, ns: &resolver::Contract, expr: &Expression) -> String {
        match expr {
            Expression::BoolLiteral(false) => "false".to_string(),
            Expression::BoolLiteral(true) => "true".to_string(),
            Expression::BytesLiteral(s) => format!("hex\"{}\"", hex::encode(s)),
            Expression::NumberLiteral(bits, n) => format!("i{} {}", bits, n.to_str_radix(10)),
            Expression::Add(l, r) => format!(
                "({} + {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::Subtract(l, r) => format!(
                "({} - {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::BitwiseOr(l, r) => format!(
                "({} | {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::BitwiseAnd(l, r) => format!(
                "({} & {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::BitwiseXor(l, r) => format!(
                "({} ^ {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::ShiftLeft(l, r) => format!(
                "({} << {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::ShiftRight(l, r, _) => format!(
                "({} >> {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::Multiply(l, r) => format!(
                "({} * {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::UDivide(l, r) | Expression::SDivide(l, r) => format!(
                "({} / {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::UModulo(l, r) | Expression::SModulo(l, r) => format!(
                "({} % {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::Power(l, r) => format!(
                "({} ** {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::Variable(_, res) => format!("%{}", self.vars[*res].id.name),

            Expression::ZeroExt(ty, e) => {
                format!("(zext {} {})", ty.to_string(ns), self.expr_to_string(ns, e))
            }
            Expression::SignExt(ty, e) => {
                format!("(sext {} {})", ty.to_string(ns), self.expr_to_string(ns, e))
            }
            Expression::Trunc(ty, e) => format!(
                "(trunc {} {})",
                ty.to_string(ns),
                self.expr_to_string(ns, e)
            ),
            Expression::SMore(l, r) => format!(
                "({} >(s) {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::SLess(l, r) => format!(
                "({} <(s) {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::SMoreEqual(l, r) => format!(
                "({} >=(s) {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::SLessEqual(l, r) => format!(
                "({} <=(s) {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::UMore(l, r) => format!(
                "({} >(u) {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::ULess(l, r) => format!(
                "({} <(u) {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::UMoreEqual(l, r) => format!(
                "({} >=(u) {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::ULessEqual(l, r) => format!(
                "({} <=(u) {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::Equal(l, r) => format!(
                "({} = {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::NotEqual(l, r) => format!(
                "({} != {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::IndexAccess(a, i) => {
                format!("%{}[{}]", self.vars[*a].id.name, self.expr_to_string(ns, i))
            }
            Expression::Or(l, r) => format!(
                "({} || {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::And(l, r) => format!(
                "({} && {})",
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::Ternary(c, l, r) => format!(
                "({} ? {} : {})",
                self.expr_to_string(ns, c),
                self.expr_to_string(ns, l),
                self.expr_to_string(ns, r)
            ),
            Expression::Not(e) => format!("!{}", self.expr_to_string(ns, e)),
            Expression::Complement(e) => format!("~{}", self.expr_to_string(ns, e)),
            Expression::UnaryMinus(e) => format!("-{}", self.expr_to_string(ns, e)),
            Expression::Poison => "☠".to_string(),
        }
    }

    pub fn instr_to_string(&self, ns: &resolver::Contract, instr: &Instr) -> String {
        match instr {
            Instr::Return { value } => {
                let mut s = String::from("return ");
                let mut first = true;

                for arg in value {
                    if !first {
                        s.push_str(", ");
                    }
                    first = false;
                    s.push_str(&self.expr_to_string(ns, arg));
                }

                s
            }
            Instr::Set { res, expr } => format!(
                "%{} = {}",
                self.vars[*res].id.name,
                self.expr_to_string(ns, expr)
            ),
            Instr::Constant { res, constant } => format!(
                "%{} = const {}",
                self.vars[*res].id.name,
                self.expr_to_string(ns, &ns.constants[*constant])
            ),
            Instr::Branch { bb } => format!("branch bb{}", bb),
            Instr::BranchCond {
                cond,
                true_,
                false_,
            } => format!(
                "branchcond {}, bb{}, bb{}",
                self.expr_to_string(ns, cond),
                true_,
                false_
            ),
            Instr::FuncArg { res, arg } => {
                format!("%{} = funcarg({})", self.vars[*res].id.name, arg)
            }
            Instr::SetStorage { local, storage } => {
                format!("setstorage %{} = %{}", *storage, self.vars[*local].id.name)
            }
            Instr::GetStorage { local, storage } => {
                format!("getstorage %{} = %{}", *storage, self.vars[*local].id.name)
            }
            Instr::AssertFailure {} => "assert-failure".to_string(),
            Instr::Call { res, func, args } => format!(
                "{} = call {} {} {}",
                {
                    let s: Vec<String> = res
                        .iter()
                        .map(|local| format!("%{}", self.vars[*local].id.name))
                        .collect();

                    s.join(", ")
                },
                *func,
                ns.functions[*func].name.to_owned(),
                {
                    let s: Vec<String> = args
                        .iter()
                        .map(|expr| self.expr_to_string(ns, expr))
                        .collect();

                    s.join(", ")
                }
            ),
        }
    }

    pub fn basic_block_to_string(&self, ns: &resolver::Contract, pos: usize) -> String {
        let mut s = format!("bb{}: # {}\n", pos, self.bb[pos].name);

        if let Some(ref phis) = self.bb[pos].phis {
            s.push_str("# phis: ");
            let mut first = true;
            for p in phis {
                if !first {
                    s.push_str(", ");
                }
                first = false;
                s.push_str(&self.vars[*p].id.name);
            }
            s.push_str("\n");
        }

        for ins in &self.bb[pos].instr {
            s.push_str(&format!("\t{}\n", self.instr_to_string(ns, ins)));
        }

        s
    }

    pub fn to_string(&self, ns: &resolver::Contract) -> String {
        let mut s = String::from("");

        for i in 0..self.bb.len() {
            s.push_str(&self.basic_block_to_string(ns, i));
        }

        s
    }
}

pub fn generate_cfg(
    ast_f: &ast::FunctionDefinition,
    resolve_f: &resolver::FunctionDecl,
    ns: &resolver::Contract,
    errors: &mut Vec<output::Output>,
) -> Result<Box<ControlFlowGraph>, ()> {
    let mut cfg = Box::new(ControlFlowGraph::new());

    let mut vartab = Vartable::new();
    let mut loops = LoopScopes::new();

    // first add function parameters
    for (i, p) in ast_f.params.iter().enumerate() {
        if let Some(ref name) = p.name {
            if let Some(pos) = vartab.add(name, resolve_f.params[i].ty.clone(), errors) {
                ns.check_shadowing(name, errors);

                cfg.add(&mut vartab, Instr::FuncArg { res: pos, arg: i });
            }
        }
    }

    // If any of the return values are named, then the return statement can be omitted at
    // the end of the function, and return values may be omitted too. Create variables to
    // store the return values
    if ast_f.returns.iter().any(|v| v.name.is_some()) {
        let mut returns = Vec::new();

        for (i, p) in ast_f.returns.iter().enumerate() {
            returns.push(if let Some(ref name) = p.name {
                if let Some(pos) = vartab.add(name, resolve_f.returns[i].ty.clone(), errors) {
                    ns.check_shadowing(name, errors);

                    // set to zero
                    cfg.add(
                        &mut vartab,
                        Instr::Set {
                            res: pos,
                            expr: resolve_f.returns[i].ty.default(ns),
                        },
                    );

                    pos
                } else {
                    // obs wrong but we had an error so will continue with bogus value to generate parser errors
                    0
                }
            } else {
                // this variable can never be assigned but will need a zero value
                let pos = vartab.temp(
                    &ast::Identifier {
                        loc: ast::Loc(0, 0),
                        name: format!("arg{}", i),
                    },
                    &resolve_f.returns[i].ty.clone(),
                );

                // set to zero
                cfg.add(
                    &mut vartab,
                    Instr::Set {
                        res: pos,
                        expr: resolve_f.returns[i].ty.default(ns),
                    },
                );

                pos
            });
        }

        vartab.returns = returns;
    }

    let reachable = statement(
        &ast_f.body,
        resolve_f,
        &mut cfg,
        ns,
        &mut vartab,
        &mut loops,
        errors,
    )?;

    // ensure we have a return instruction
    if reachable {
        check_return(ast_f, &mut cfg, &vartab, errors)?;
    }

    cfg.vars = vartab.drain();

    // walk cfg to check for use for before initialize

    Ok(cfg)
}

fn check_return(
    f: &ast::FunctionDefinition,
    cfg: &mut ControlFlowGraph,
    vartab: &Vartable,
    errors: &mut Vec<output::Output>,
) -> Result<(), ()> {
    let current = cfg.current;
    let bb = &mut cfg.bb[current];

    let num_instr = bb.instr.len();

    if num_instr > 0 {
        if let Instr::Return { .. } = bb.instr[num_instr - 1] {
            return Ok(());
        }
    }

    if f.returns.is_empty() || !vartab.returns.is_empty() {
        bb.add(Instr::Return {
            value: vartab
                .returns
                .iter()
                .map(|pos| Expression::Variable(ast::Loc(0, 0), *pos))
                .collect(),
        });

        Ok(())
    } else {
        errors.push(Output::error(
            f.body.loc(),
            "missing return statement".to_string(),
        ));
        Err(())
    }
}

pub fn get_contract_storage(var: &Variable, cfg: &mut ControlFlowGraph, vartab: &mut Vartable) {
    match var.storage {
        Storage::Contract(offset) => {
            cfg.reads_contract_storage = true;
            cfg.add(
                vartab,
                Instr::GetStorage {
                    local: var.pos,
                    storage: offset,
                },
            );
        }
        Storage::Constant(n) => {
            cfg.add(
                vartab,
                Instr::Constant {
                    res: var.pos,
                    constant: n,
                },
            );
        }
        Storage::Local => (),
    }
}

pub fn set_contract_storage(
    id: &ast::Identifier,
    var: &Variable,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    errors: &mut Vec<output::Output>,
) -> Result<(), ()> {
    match var.storage {
        Storage::Contract(offset) => {
            cfg.writes_contract_storage = true;
            cfg.add(
                vartab,
                Instr::SetStorage {
                    local: var.pos,
                    storage: offset,
                },
            );

            Ok(())
        }
        Storage::Constant(_) => {
            errors.push(Output::type_error(
                id.loc,
                format!("cannot assign to constant {}", id.name),
            ));
            Err(())
        }
        Storage::Local => {
            // nothing to do
            Ok(())
        }
    }
}

fn statement(
    stmt: &ast::Statement,
    f: &resolver::FunctionDecl,
    cfg: &mut ControlFlowGraph,
    ns: &resolver::Contract,
    vartab: &mut Vartable,
    loops: &mut LoopScopes,
    errors: &mut Vec<output::Output>,
) -> Result<bool, ()> {
    match stmt {
        ast::Statement::VariableDefinition(decl, init) => {
            let var_ty = ns.resolve_type(&decl.typ, Some(errors))?;

            if var_ty.size_hint() > BigInt::from(1024 * 1024) {
                errors.push(Output::error(
                    stmt.loc(),
                    "type to large to fit into memory".to_string(),
                ));
                return Err(());
            }

            let e_t = if let Some(init) = init {
                let (expr, init_ty) = expression(init, cfg, ns, &mut Some(vartab), errors)?;

                Some(cast(
                    &decl.name.loc,
                    expr,
                    &init_ty,
                    &var_ty,
                    true,
                    ns,
                    errors,
                )?)
            } else {
                None
            };

            if let Some(pos) = vartab.add(&decl.name, var_ty, errors) {
                ns.check_shadowing(&decl.name, errors);

                if let Some(expr) = e_t {
                    cfg.add(vartab, Instr::Set { res: pos, expr });
                }
            }
            Ok(true)
        }
        ast::Statement::BlockStatement(ast::BlockStatement(bs)) => {
            vartab.new_scope();
            let mut reachable = true;

            for stmt in bs {
                if !reachable {
                    errors.push(Output::error(
                        stmt.loc(),
                        "unreachable statement".to_string(),
                    ));
                    return Err(());
                }
                reachable = statement(&stmt, f, cfg, ns, vartab, loops, errors)?;
            }

            vartab.leave_scope();

            Ok(reachable)
        }
        ast::Statement::Return(loc, returns) if returns.is_empty() => {
            let no_returns = f.returns.len();

            if vartab.returns.len() != no_returns {
                errors.push(Output::error(
                    *loc,
                    format!(
                        "missing return value, {} return values expected",
                        no_returns
                    ),
                ));
                return Err(());
            }

            cfg.add(
                vartab,
                Instr::Return {
                    value: vartab
                        .returns
                        .iter()
                        .map(|pos| Expression::Variable(ast::Loc(0, 0), *pos))
                        .collect(),
                },
            );

            Ok(false)
        }
        ast::Statement::Return(loc, returns) => {
            let no_returns = f.returns.len();

            if no_returns > 0 && returns.is_empty() {
                errors.push(Output::error(
                    *loc,
                    format!(
                        "missing return value, {} return values expected",
                        no_returns
                    ),
                ));
                return Err(());
            }

            if no_returns == 0 && !returns.is_empty() {
                errors.push(Output::error(
                    *loc,
                    "function has no return values".to_string(),
                ));
                return Err(());
            }

            if no_returns != returns.len() {
                errors.push(Output::error(
                    *loc,
                    format!(
                        "incorrect number of return values, expected {} but got {}",
                        no_returns,
                        returns.len()
                    ),
                ));
                return Err(());
            }

            let mut exprs = Vec::new();

            for (i, r) in returns.iter().enumerate() {
                let (e, ty) = expression(r, cfg, ns, &mut Some(vartab), errors)?;

                exprs.push(cast(&r.loc(), e, &ty, &f.returns[i].ty, true, ns, errors)?);
            }

            cfg.add(vartab, Instr::Return { value: exprs });

            Ok(false)
        }
        ast::Statement::Expression(expr) => {
            expression(expr, cfg, ns, &mut Some(vartab), errors)?;

            Ok(true)
        }
        ast::Statement::If(cond, then_stmt, None) => {
            let (expr, expr_ty) = expression(cond, cfg, ns, &mut Some(vartab), errors)?;

            let then = cfg.new_basic_block("then".to_string());
            let endif = cfg.new_basic_block("endif".to_string());

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: cast(
                        &cond.loc(),
                        expr,
                        &expr_ty,
                        &resolver::Type::new_bool(),
                        true,
                        ns,
                        errors,
                    )?,
                    true_: then,
                    false_: endif,
                },
            );

            cfg.set_basic_block(then);

            vartab.new_scope();
            vartab.new_dirty_tracker();

            let reachable = statement(then_stmt, f, cfg, ns, vartab, loops, errors)?;

            if reachable {
                cfg.add(vartab, Instr::Branch { bb: endif });
            }

            vartab.leave_scope();
            cfg.set_phis(endif, vartab.pop_dirty_tracker());

            cfg.set_basic_block(endif);

            Ok(true)
        }
        ast::Statement::If(cond, then_stmt, Some(else_stmt)) => {
            let (expr, expr_ty) = expression(cond, cfg, ns, &mut Some(vartab), errors)?;

            let then = cfg.new_basic_block("then".to_string());
            let else_ = cfg.new_basic_block("else".to_string());
            let endif = cfg.new_basic_block("endif".to_string());

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: cast(
                        &cond.loc(),
                        expr,
                        &expr_ty,
                        &resolver::Type::new_bool(),
                        true,
                        ns,
                        errors,
                    )?,
                    true_: then,
                    false_: else_,
                },
            );

            // then
            cfg.set_basic_block(then);

            vartab.new_scope();
            vartab.new_dirty_tracker();

            let then_reachable = statement(then_stmt, f, cfg, ns, vartab, loops, errors)?;

            if then_reachable {
                cfg.add(vartab, Instr::Branch { bb: endif });
            }

            vartab.leave_scope();

            // else
            cfg.set_basic_block(else_);

            vartab.new_scope();

            let else_reachable = statement(else_stmt, f, cfg, ns, vartab, loops, errors)?;

            if else_reachable {
                cfg.add(vartab, Instr::Branch { bb: endif });
            }

            vartab.leave_scope();
            cfg.set_phis(endif, vartab.pop_dirty_tracker());

            cfg.set_basic_block(endif);

            Ok(then_reachable || else_reachable)
        }
        ast::Statement::Break => match loops.do_break() {
            Some(bb) => {
                cfg.add(vartab, Instr::Branch { bb });
                Ok(false)
            }
            None => {
                errors.push(Output::error(
                    stmt.loc(),
                    "break statement not in loop".to_string(),
                ));
                Err(())
            }
        },
        ast::Statement::Continue => match loops.do_continue() {
            Some(bb) => {
                cfg.add(vartab, Instr::Branch { bb });
                Ok(false)
            }
            None => {
                errors.push(Output::error(
                    stmt.loc(),
                    "continue statement not in loop".to_string(),
                ));
                Err(())
            }
        },
        ast::Statement::DoWhile(body_stmt, cond_expr) => {
            let body = cfg.new_basic_block("body".to_string());
            let cond = cfg.new_basic_block("conf".to_string());
            let end = cfg.new_basic_block("enddowhile".to_string());

            cfg.add(vartab, Instr::Branch { bb: body });

            cfg.set_basic_block(body);

            vartab.new_scope();
            vartab.new_dirty_tracker();
            loops.new_scope(end, cond);

            let mut body_reachable = statement(body_stmt, f, cfg, ns, vartab, loops, errors)?;

            if body_reachable {
                cfg.add(vartab, Instr::Branch { bb: cond });
            }

            vartab.leave_scope();
            let control = loops.leave_scope();

            if control.no_continues > 0 {
                body_reachable = true
            }

            if body_reachable {
                cfg.set_basic_block(cond);

                let (expr, expr_ty) = expression(cond_expr, cfg, ns, &mut Some(vartab), errors)?;

                cfg.add(
                    vartab,
                    Instr::BranchCond {
                        cond: cast(
                            &cond_expr.loc(),
                            expr,
                            &expr_ty,
                            &resolver::Type::new_bool(),
                            true,
                            ns,
                            errors,
                        )?,
                        true_: body,
                        false_: end,
                    },
                );
            }

            let set = vartab.pop_dirty_tracker();
            cfg.set_phis(end, set.clone());
            cfg.set_phis(body, set.clone());
            cfg.set_phis(cond, set);

            cfg.set_basic_block(end);

            Ok(body_reachable || control.no_breaks > 0)
        }
        ast::Statement::While(cond_expr, body_stmt) => {
            let cond = cfg.new_basic_block("cond".to_string());
            let body = cfg.new_basic_block("body".to_string());
            let end = cfg.new_basic_block("endwhile".to_string());

            cfg.add(vartab, Instr::Branch { bb: cond });

            cfg.set_basic_block(cond);

            let (expr, expr_ty) = expression(cond_expr, cfg, ns, &mut Some(vartab), errors)?;

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: cast(
                        &cond_expr.loc(),
                        expr,
                        &expr_ty,
                        &resolver::Type::new_bool(),
                        true,
                        ns,
                        errors,
                    )?,
                    true_: body,
                    false_: end,
                },
            );

            cfg.set_basic_block(body);

            vartab.new_scope();
            vartab.new_dirty_tracker();
            loops.new_scope(end, cond);

            let body_reachable = statement(body_stmt, f, cfg, ns, vartab, loops, errors)?;

            if body_reachable {
                cfg.add(vartab, Instr::Branch { bb: cond });
            }

            vartab.leave_scope();
            loops.leave_scope();
            let set = vartab.pop_dirty_tracker();
            cfg.set_phis(end, set.clone());
            cfg.set_phis(cond, set);

            cfg.set_basic_block(end);

            Ok(true)
        }
        ast::Statement::For(init_stmt, None, next_stmt, body_stmt) => {
            let body = cfg.new_basic_block("body".to_string());
            let next = cfg.new_basic_block("next".to_string());
            let end = cfg.new_basic_block("endfor".to_string());

            vartab.new_scope();

            if let Some(init_stmt) = init_stmt {
                statement(init_stmt, f, cfg, ns, vartab, loops, errors)?;
            }

            cfg.add(vartab, Instr::Branch { bb: body });

            cfg.set_basic_block(body);

            loops.new_scope(
                end,
                match next_stmt {
                    Some(_) => next,
                    None => body,
                },
            );
            vartab.new_dirty_tracker();

            let mut body_reachable = match body_stmt {
                Some(body_stmt) => statement(body_stmt, f, cfg, ns, vartab, loops, errors)?,
                None => true,
            };

            if body_reachable {
                cfg.add(vartab, Instr::Branch { bb: next });
            }

            let control = loops.leave_scope();

            if control.no_continues > 0 {
                body_reachable = true;
            }

            if body_reachable {
                if let Some(next_stmt) = next_stmt {
                    cfg.set_basic_block(next);
                    body_reachable = statement(next_stmt, f, cfg, ns, vartab, loops, errors)?;
                }

                if body_reachable {
                    cfg.add(vartab, Instr::Branch { bb: body });
                }
            }

            let set = vartab.pop_dirty_tracker();
            if control.no_continues > 0 {
                cfg.set_phis(next, set.clone());
            }
            cfg.set_phis(body, set.clone());
            cfg.set_phis(end, set);

            vartab.leave_scope();
            cfg.set_basic_block(end);

            Ok(control.no_breaks > 0)
        }
        ast::Statement::For(init_stmt, Some(cond_expr), next_stmt, body_stmt) => {
            let body = cfg.new_basic_block("body".to_string());
            let cond = cfg.new_basic_block("cond".to_string());
            let next = cfg.new_basic_block("next".to_string());
            let end = cfg.new_basic_block("endfor".to_string());

            vartab.new_scope();

            if let Some(init_stmt) = init_stmt {
                statement(init_stmt, f, cfg, ns, vartab, loops, errors)?;
            }

            cfg.add(vartab, Instr::Branch { bb: cond });

            cfg.set_basic_block(cond);

            let (expr, expr_ty) = expression(cond_expr, cfg, ns, &mut Some(vartab), errors)?;

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: cast(
                        &cond_expr.loc(),
                        expr,
                        &expr_ty,
                        &resolver::Type::new_bool(),
                        true,
                        ns,
                        errors,
                    )?,
                    true_: body,
                    false_: end,
                },
            );

            cfg.set_basic_block(body);

            // continue goes to next, and if that does exist, cond
            loops.new_scope(
                end,
                match next_stmt {
                    Some(_) => next,
                    None => cond,
                },
            );
            vartab.new_dirty_tracker();

            let mut body_reachable = match body_stmt {
                Some(body_stmt) => statement(body_stmt, f, cfg, ns, vartab, loops, errors)?,
                None => true,
            };

            if body_reachable {
                cfg.add(vartab, Instr::Branch { bb: next });
            }

            let control = loops.leave_scope();

            if control.no_continues > 0 {
                body_reachable = true;
            }

            if body_reachable {
                if let Some(next_stmt) = next_stmt {
                    cfg.set_basic_block(next);
                    body_reachable = statement(next_stmt, f, cfg, ns, vartab, loops, errors)?;
                }

                if body_reachable {
                    cfg.add(vartab, Instr::Branch { bb: cond });
                }
            }

            vartab.leave_scope();
            cfg.set_basic_block(end);

            let set = vartab.pop_dirty_tracker();
            if control.no_continues > 0 {
                cfg.set_phis(next, set.clone());
            }
            if control.no_breaks > 0 {
                cfg.set_phis(end, set.clone());
            }
            cfg.set_phis(cond, set);

            Ok(true)
        }
        _ => panic!("not implemented"),
    }
}

// Vartable
// methods
// create variable with loc, name, Type -> pos
// find variable by name -> Some(pos)
// new scope
// leave scope
// produce full Vector of all variables
#[derive(Clone)]
pub enum Storage {
    Constant(usize),
    Contract(usize),
    Local,
}

#[derive(Clone)]
pub struct Variable {
    pub id: ast::Identifier,
    pub ty: resolver::Type,
    pub pos: usize,
    pub storage: Storage,
}

struct VarScope(HashMap<String, usize>, Option<HashSet<usize>>);

#[derive(Default)]
pub struct Vartable {
    vars: Vec<Variable>,
    names: LinkedList<VarScope>,
    storage_vars: HashMap<String, usize>,
    dirty: Vec<DirtyTracker>,
    returns: Vec<usize>,
}

pub struct DirtyTracker {
    lim: usize,
    set: HashSet<usize>,
}

impl Vartable {
    pub fn new() -> Self {
        let mut list = LinkedList::new();
        list.push_front(VarScope(HashMap::new(), None));
        Vartable {
            vars: Vec::new(),
            names: list,
            storage_vars: HashMap::new(),
            dirty: Vec::new(),
            returns: Vec::new(),
        }
    }

    pub fn add(
        &mut self,
        id: &ast::Identifier,
        ty: resolver::Type,
        errors: &mut Vec<output::Output>,
    ) -> Option<usize> {
        if let Some(ref prev) = self.find_local(&id.name) {
            errors.push(Output::error_with_note(
                id.loc,
                format!("{} is already declared", id.name.to_string()),
                prev.id.loc,
                "location of previous declaration".to_string(),
            ));
            return None;
        }

        let pos = self.vars.len();

        self.vars.push(Variable {
            id: id.clone(),
            ty,
            pos,
            storage: Storage::Local,
        });

        self.names
            .front_mut()
            .unwrap()
            .0
            .insert(id.name.to_string(), pos);

        Some(pos)
    }

    fn find_local(&self, name: &str) -> Option<&Variable> {
        for scope in &self.names {
            if let Some(n) = scope.0.get(name) {
                return Some(&self.vars[*n]);
            }
        }

        None
    }

    pub fn find(
        &mut self,
        id: &ast::Identifier,
        contract: &resolver::Contract,
        errors: &mut Vec<output::Output>,
    ) -> Result<Variable, ()> {
        for scope in &self.names {
            if let Some(n) = scope.0.get(&id.name) {
                return Ok(self.vars[*n].clone());
            }
        }

        if let Some(n) = self.storage_vars.get(&id.name) {
            return Ok(self.vars[*n].clone());
        }

        let v = contract.resolve_var(&id, errors)?;
        let var = &contract.variables[v];
        let pos = self.vars.len();

        self.vars.push(Variable {
            id: id.clone(),
            ty: var.ty.clone(),
            pos,
            storage: match var.var {
                resolver::ContractVariableType::Storage(n) => Storage::Contract(n),
                resolver::ContractVariableType::Constant(n) => Storage::Constant(n),
            },
        });

        self.storage_vars.insert(id.name.to_string(), pos);

        Ok(self.vars[pos].clone())
    }

    pub fn temp(&mut self, id: &ast::Identifier, ty: &resolver::Type) -> usize {
        let pos = self.vars.len();

        self.vars.push(Variable {
            id: ast::Identifier {
                name: format!("{}.temp.{}", id.name, pos),
                loc: id.loc,
            },
            ty: ty.clone(),
            pos,
            storage: Storage::Local,
        });

        pos
    }

    pub fn new_scope(&mut self) {
        self.names.push_front(VarScope(HashMap::new(), None));
    }

    pub fn leave_scope(&mut self) {
        self.names.pop_front();
    }

    pub fn drain(self) -> Vec<Variable> {
        self.vars
    }

    // In order to create phi nodes, we need to track what vars are set in a certain scope
    pub fn set_dirty(&mut self, pos: usize) {
        for e in &mut self.dirty {
            if pos < e.lim {
                e.set.insert(pos);
            }
        }
    }

    pub fn new_dirty_tracker(&mut self) {
        self.dirty.push(DirtyTracker {
            lim: self.vars.len(),
            set: HashSet::new(),
        });
    }

    pub fn pop_dirty_tracker(&mut self) -> HashSet<usize> {
        self.dirty.pop().unwrap().set
    }
}

struct LoopScope {
    break_bb: usize,
    continue_bb: usize,
    no_breaks: usize,
    no_continues: usize,
}

struct LoopScopes(LinkedList<LoopScope>);

impl LoopScopes {
    fn new() -> Self {
        LoopScopes(LinkedList::new())
    }

    fn new_scope(&mut self, break_bb: usize, continue_bb: usize) {
        self.0.push_front(LoopScope {
            break_bb,
            continue_bb,
            no_breaks: 0,
            no_continues: 0,
        })
    }

    fn leave_scope(&mut self) -> LoopScope {
        self.0.pop_front().unwrap()
    }

    fn do_break(&mut self) -> Option<usize> {
        match self.0.front_mut() {
            Some(scope) => {
                scope.no_breaks += 1;
                Some(scope.break_bb)
            }
            None => None,
        }
    }

    fn do_continue(&mut self) -> Option<usize> {
        match self.0.front_mut() {
            Some(scope) => {
                scope.no_continues += 1;
                Some(scope.continue_bb)
            }
            None => None,
        }
    }
}

impl resolver::Type {
    fn default(&self, ns: &resolver::Contract) -> Expression {
        match self {
            resolver::Type::Primitive(e) => e.default(),
            resolver::Type::Enum(e) => ns.enums[*e].ty.default(),
            resolver::Type::Noreturn => unreachable!(),
            resolver::Type::FixedArray(_, _) => unreachable!(),
        }
    }
}

impl ast::PrimitiveType {
    fn default(self) -> Expression {
        match self {
            ast::PrimitiveType::Uint(b) | ast::PrimitiveType::Int(b) => {
                Expression::NumberLiteral(b, BigInt::from(0))
            }
            ast::PrimitiveType::Bool => Expression::BoolLiteral(false),
            ast::PrimitiveType::Address => Expression::NumberLiteral(160, BigInt::from(0)),
            ast::PrimitiveType::Bytes(n) => {
                let mut l = Vec::new();
                l.resize(n as usize, 0);
                Expression::BytesLiteral(l)
            }
            ast::PrimitiveType::DynamicBytes => unimplemented!(),
            ast::PrimitiveType::String => unimplemented!(),
        }
    }
}
