

use std::cmp;
use num_bigint::BigInt;
use num_bigint::Sign;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::LinkedList;
use num_traits::One;
use unescape::unescape;

use ast;
use hex;
use resolver;
use output;
use output::Output;

pub enum Expression {
    BoolLiteral(bool),
    StringLiteral(String),
    HexLiteral(Vec<u8>),
    NumberLiteral(u16, BigInt),
    Add(Box<Expression>, Box<Expression>),
    Subtract(Box<Expression>, Box<Expression>),
    Multiply(Box<Expression>, Box<Expression>),
    UDivide(Box<Expression>, Box<Expression>),
    SDivide(Box<Expression>, Box<Expression>),
    UModulo(Box<Expression>, Box<Expression>),
    SModulo(Box<Expression>, Box<Expression>),
    Variable(ast::Loc, usize),
    ZeroExt(resolver::TypeName, Box<Expression>),
    SignExt(resolver::TypeName, Box<Expression>),

    More(Box<Expression>, Box<Expression>),
    Less(Box<Expression>, Box<Expression>),
    MoreEqual(Box<Expression>, Box<Expression>),
    LessEqual(Box<Expression>, Box<Expression>),
    Equal(Box<Expression>, Box<Expression>),
    NotEqual(Box<Expression>, Box<Expression>),

    Not(Box<Expression>),
    Complement(Box<Expression>),
    UnaryMinus(Box<Expression>),
}

pub enum Instr {
    FuncArg{ res: usize, arg: usize },
    Set{ res: usize, expr: Expression },
    Return{ value: Vec<Expression> },
    Branch{ bb: usize },
    BranchCond{ cond:  Expression, true_: usize, false_: usize }
}

pub struct BasicBlock {
    pub phis: Option<HashSet<usize>>,
    pub name: String,
    pub instr: Vec<Instr>
}

impl BasicBlock {
    pub fn add(&mut self, ins: Instr) {
        self.instr.push(ins);
    }
}

pub struct ControlFlowGraph {
    pub vars: Vec<Variable>,
    pub bb: Vec<BasicBlock>,
    current: usize,
}

impl ControlFlowGraph {
    pub fn new_basic_block(&mut self, name: String) -> usize {
        let pos = self.bb.len();

        self.bb.push(BasicBlock{
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
        if let Instr::Set{ res, .. } = ins {
            vartab.set_dirty(res);
        }
        self.bb[self.current].add(ins);
    }

    pub fn expr_to_string(&self, ns: &resolver::ContractNameSpace, expr: &Expression) -> String {
        match expr {
            Expression::BoolLiteral(false) => "false".to_string(),
            Expression::BoolLiteral(true) => "true".to_string(),
            Expression::StringLiteral(s) => format!("\"{}\"", s), // FIXME: escape with lion snailquote
            Expression::HexLiteral(s) => format!("hex\"{}\"", hex::encode(s)),
            Expression::NumberLiteral(bits, n) => format!("i{} {}", bits, n.to_str_radix(10)),
            Expression::Add(l, r) => format!("({} + {})", self.expr_to_string(ns, l), self.expr_to_string(ns, r)),
            Expression::Subtract(l, r) => format!("({} - {})", self.expr_to_string(ns, l), self.expr_to_string(ns, r)),
            Expression::Multiply(l, r) => format!("({} * {})", self.expr_to_string(ns, l), self.expr_to_string(ns, r)),
            Expression::UDivide(l, r) |
            Expression::SDivide(l, r) => format!("({} / {})", self.expr_to_string(ns, l), self.expr_to_string(ns, r)),
            Expression::UModulo(l, r) |
            Expression::SModulo(l, r) => format!("({} % {})", self.expr_to_string(ns, l), self.expr_to_string(ns, r)),
            Expression::Variable(_, res) => format!("%{}", self.vars[*res].id.name),

            Expression::ZeroExt(ty, e) => format!("(zext {} {})", ty.to_string(ns), self.expr_to_string(ns, e)),
            Expression::SignExt(ty, e) => format!("(sext {} {})", ty.to_string(ns), self.expr_to_string(ns, e)),

            Expression::More(l, r) => format!("({} > {})", self.expr_to_string(ns, l), self.expr_to_string(ns, r)),
            Expression::Less(l, r) => format!("({} < {})", self.expr_to_string(ns, l), self.expr_to_string(ns, r)),
            Expression::MoreEqual(l, r) => format!("({} >= {})", self.expr_to_string(ns, l), self.expr_to_string(ns, r)),
            Expression::Equal(l, r) => format!("({} = {})", self.expr_to_string(ns, l), self.expr_to_string(ns, r)),
            Expression::NotEqual(l, r) => format!("({} != {})", self.expr_to_string(ns, l), self.expr_to_string(ns, r)),


            Expression::Not(e) => format!("!{}", self.expr_to_string(ns, e)),
            Expression::Complement(e) => format!("~{}", self.expr_to_string(ns, e)),
            Expression::UnaryMinus(e) => format!("-{}", self.expr_to_string(ns, e)),
            _ => String::from("")
        }
    }

    pub fn instr_to_string(&self, ns: &resolver::ContractNameSpace, instr: &Instr) -> String {
        match instr {
            Instr::Return{ value } => {
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
            },
            Instr::Set{ res, expr } => format!("%{} = {}", self.vars[*res].id.name, self.expr_to_string(ns, expr)),
            Instr::Branch{ bb } => format!("branch bb{}", bb),
            Instr::BranchCond{ cond, true_, false_ } => format!("branchcond {}, bb{}, bb{}", self.expr_to_string(ns, cond), true_, false_),
            Instr::FuncArg{ res, arg } => format!("%{} = funcarg({})", self.vars[*res].id.name, arg),
        }
    }

    pub fn basic_block_to_string(&self, ns: &resolver::ContractNameSpace, pos: usize) -> String {
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

    pub fn to_string(&self, ns: &resolver::ContractNameSpace) -> String {
        let mut s = String::from("");

        for i in 0..self.bb.len() {
            s.push_str(&self.basic_block_to_string(ns, i));
        }

        s
    }
}

pub fn generate_cfg(ast_f: &ast::FunctionDefinition, resolve_f: &resolver::FunctionDecl, ns: &resolver::ContractNameSpace, errors: &mut Vec<output::Output>) -> Result<Box<ControlFlowGraph>, ()> {
    let mut cfg = Box::new(ControlFlowGraph{
        vars: Vec::new(),
        bb: Vec::new(),
        current: 0
    });

    cfg.new_basic_block("entry".to_string());

    let mut vartab = Vartable::new();
    let mut loops = LoopScopes::new();
  
    // first add function parameters
    for (i, p) in ast_f.params.iter().enumerate() {
        if let Some(ref name) = p.name {
            if let Some(pos) = vartab.add(name, resolve_f.params[i].ty.clone(), errors) {
                ns.check_shadowing(name, errors);

                cfg.add(&mut vartab, Instr::FuncArg{
                    res: pos,
                    arg: i
                });
            }
        }
    }

    let reachable = statement(&ast_f.body, resolve_f, &mut cfg, ns, &mut vartab, &mut loops, errors)?;

    cfg.vars = vartab.drain();

    // ensure we have a return instruction
    if reachable {
        check_return(ast_f, &mut cfg, errors)?;
    }

    // walk cfg to check for use for before initialize

    Ok(cfg)
}

fn check_return(f: &ast::FunctionDefinition, cfg: &mut ControlFlowGraph, errors: &mut Vec<output::Output>) -> Result<(), ()> {
    let current = cfg.current;
    let bb = &mut cfg.bb[current];

    let num_instr = bb.instr.len();

    if num_instr > 0 {
        if let Instr::Return{ .. } = bb.instr[num_instr - 1] {
            return Ok(())
        }
    }

    if f.returns.is_empty() {
        bb.add(Instr::Return{
            value: Vec::new()
        });

        Ok(())
    } else {
        errors.push(Output::error(f.body.loc(), format!("missing return statement")));
        return Err(());
    }
}

fn statement(stmt: &ast::Statement, f: &resolver::FunctionDecl, cfg: &mut ControlFlowGraph, ns: &resolver::ContractNameSpace, vartab: &mut Vartable, loops: &mut LoopScopes, errors: &mut Vec<output::Output>) -> Result<bool, ()> {
    match stmt {
        ast::Statement::VariableDefinition(decl, init) => {
            let var_ty = match ns.resolve(&decl.typ, errors) {
                Some(ty) => ty,
                None => return Err(())
            };

            let e_t = if let Some(init) = init {
                let (expr, init_ty) = expression(init, cfg, ns, vartab, errors)?;

                Some(implicit_cast(
                    &decl.name.loc, expr,
                    &init_ty, &var_ty, ns, errors)?)
            } else {
                None
            };

            if let Some(pos) = vartab.add(&decl.name, var_ty, errors) {
                ns.check_shadowing(&decl.name, errors);

                if let Some(expr) = e_t {
                    cfg.add(vartab, Instr::Set{
                        res: pos,
                        expr: expr
                    });
                }
            }
            Ok(true)
        },
        ast::Statement::BlockStatement(ast::BlockStatement(bs)) => {
            vartab.new_scope();
            let mut reachable = true;

            for stmt in bs {
                if !reachable {
                    errors.push(Output::error(stmt.loc(), format!("unreachable statement")));
                    return Err(());
                }
                reachable = statement(&stmt, f, cfg, ns, vartab, loops, errors)?;
            }

            vartab.leave_scope();

            Ok(reachable)
        },
        ast::Statement::Return(loc, returns) => {
            let no_returns = f.returns.len();

            if no_returns > 0 && returns.is_empty() {
                errors.push(Output::error(loc.clone(), format!("missing return value, {} return values expected", no_returns)));
                return Err(());
            }

            if no_returns == 0 && !returns.is_empty() {
                errors.push(Output::error(loc.clone(), format!("function has no return values")));
                return Err(());
            }

            if no_returns != returns.len() {
                errors.push(Output::error(loc.clone(), format!("incorrect number of return values, expected {} but got {}", no_returns, returns.len())));
                return Err(());
            }

            let mut exprs = Vec::new();

            for (i, r) in returns.iter().enumerate() {
                let (e, ty) = expression(r, cfg, ns, vartab, errors)?;

                exprs.push(implicit_cast(&r.loc(), e, &ty, &f.returns[i].ty, ns, errors)?);
            }

            cfg.add(vartab, Instr::Return{
                value: exprs
            });

            Ok(false)
        },
        ast::Statement::Expression(expr) => {
            expression(expr, cfg, ns, vartab, errors)?;

            Ok(true)
        },
        ast::Statement::If(cond, then_stmt, None) => {
            let (expr, expr_ty) = expression(cond, cfg, ns, vartab, errors)?;

            let then = cfg.new_basic_block("then".to_string());
            let endif = cfg.new_basic_block("endif".to_string());

            cfg.add(vartab, Instr::BranchCond{
                cond: implicit_cast(&cond.loc(), expr, &expr_ty, &resolver::TypeName::new_bool(), ns, errors)?,
                true_: then,
                false_: endif,
            });

            cfg.set_basic_block(then);

            vartab.new_scope();
            vartab.new_dirty_tracker();

            let reachable = statement(then_stmt, f, cfg, ns, vartab, loops, errors)?;

            if reachable {
                cfg.add(vartab, Instr::Branch{ bb: endif });
            }

            vartab.leave_scope();
            cfg.set_phis(endif, vartab.pop_dirty_tracker());

            cfg.set_basic_block(endif);

            Ok(true)
        },
        ast::Statement::If(cond, then_stmt, Some(else_stmt)) => {
            let (expr, expr_ty) = expression(cond, cfg, ns, vartab, errors)?;

            let then = cfg.new_basic_block("then".to_string());
            let else_ = cfg.new_basic_block("else".to_string());
            let endif = cfg.new_basic_block("endif".to_string());

            cfg.add(vartab, Instr::BranchCond{
                cond: implicit_cast(&cond.loc(), expr, &expr_ty, &resolver::TypeName::new_bool(), ns, errors)?,
                true_: then,
                false_: else_,
            });

            // then
            cfg.set_basic_block(then);

            vartab.new_scope();
            vartab.new_dirty_tracker();

            let then_reachable = statement(then_stmt, f, cfg, ns, vartab, loops, errors)?;

            if then_reachable {
                cfg.add(vartab, Instr::Branch{ bb: endif });
            }

            vartab.leave_scope();

            // else
            cfg.set_basic_block(else_);

            vartab.new_scope();

            let else_reachable = statement(else_stmt, f, cfg, ns, vartab, loops, errors)?;

            if else_reachable {
                cfg.add(vartab, Instr::Branch{ bb: endif });
            }

            vartab.leave_scope();
            cfg.set_phis(endif, vartab.pop_dirty_tracker());

            cfg.set_basic_block(endif);

            Ok(then_reachable || else_reachable)
        },
        ast::Statement::Break => {
            match loops.do_break() {
                Some(bb) => {
                    cfg.add(vartab, Instr::Branch{ bb });
                    Ok(false)
                },
                None => {
                    errors.push(Output::error(stmt.loc(), format!("break statement not in loop")));
                    Err(())
                }
            }
        },
        ast::Statement::Continue => {
            match loops.do_continue() {
                Some(bb) => {
                    cfg.add(vartab, Instr::Branch{ bb });
                    Ok(false)
                },
                None => {
                    errors.push(Output::error(stmt.loc(), format!("continue statement not in loop")));
                    Err(())
                }
            }
        },
        ast::Statement::DoWhile(body_stmt, cond_expr) => {
            let body = cfg.new_basic_block("body".to_string());
            let cond = cfg.new_basic_block("conf".to_string());
            let end = cfg.new_basic_block("enddowhile".to_string());

            cfg.add(vartab, Instr::Branch{ bb: body });

            cfg.set_basic_block(body);

            vartab.new_scope();
            vartab.new_dirty_tracker();
            loops.new_scope(end, cond);

            let mut body_reachable = statement(body_stmt, f, cfg, ns, vartab, loops, errors)?;

            if body_reachable {
                cfg.add(vartab, Instr::Branch{ bb: cond });
            }

            vartab.leave_scope();
            let control = loops.leave_scope();

            if control.no_continues > 0 {
                body_reachable = true
            }

            if body_reachable {
                cfg.set_basic_block(cond);

                let (expr, expr_ty) = expression(cond_expr, cfg, ns, vartab, errors)?;

                cfg.add(vartab, Instr::BranchCond{
                    cond: implicit_cast(&cond_expr.loc(), expr, &expr_ty, &resolver::TypeName::new_bool(), ns, errors)?,
                    true_: body,
                    false_: end,
                });
            }

            let set = vartab.pop_dirty_tracker();
            cfg.set_phis(end, set.clone());
            cfg.set_phis(body, set.clone());
            cfg.set_phis(cond, set);

            cfg.set_basic_block(end);

            Ok(body_reachable || control.no_breaks > 0)
        },
        ast::Statement::While(cond_expr, body_stmt) => {
            let cond = cfg.new_basic_block("cond".to_string());
            let body = cfg.new_basic_block("body".to_string());
            let end = cfg.new_basic_block("endwhile".to_string());

            cfg.add(vartab, Instr::Branch{ bb: cond });

            cfg.set_basic_block(cond);

            let (expr, expr_ty) = expression(cond_expr, cfg, ns, vartab, errors)?;

            cfg.add(vartab, Instr::BranchCond{
                cond: implicit_cast(&cond_expr.loc(), expr, &expr_ty, &resolver::TypeName::new_bool(), ns, errors)?,
                true_: body,
                false_: end,
            });

            cfg.set_basic_block(body);

            vartab.new_scope();
            vartab.new_dirty_tracker();
            loops.new_scope(end, cond);

            let mut body_reachable = statement(body_stmt, f, cfg, ns, vartab, loops, errors)?;

            if body_reachable {
                cfg.add(vartab, Instr::Branch{ bb: cond });
            }

            vartab.leave_scope();
            loops.leave_scope();
            let set = vartab.pop_dirty_tracker();
            cfg.set_phis(end, set.clone());
            cfg.set_phis(cond, set);

            Ok(true)
        },
        ast::Statement::For(init_stmt, None, next_stmt, body_stmt) => {
            let body = cfg.new_basic_block("body".to_string());
            let next = cfg.new_basic_block("next".to_string());
            let end = cfg.new_basic_block("endfor".to_string());

            vartab.new_scope();

            if let Some(init_stmt) = init_stmt {
                statement(init_stmt, f, cfg, ns, vartab, loops, errors)?;
            }

            cfg.add(vartab, Instr::Branch{ bb: body });

            cfg.set_basic_block(body);

            loops.new_scope(end, match next_stmt { Some(_) => next, None => body});
            vartab.new_dirty_tracker();

            let mut body_reachable = match body_stmt {
                Some(body_stmt) => statement(body_stmt, f, cfg, ns, vartab, loops, errors)?,
                None => true
            };

            if body_reachable {
                cfg.add(vartab, Instr::Branch{ bb: next });
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
                    cfg.add(vartab, Instr::Branch{ bb: body });
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
        },
        ast::Statement::For(init_stmt, Some(cond_expr), next_stmt, body_stmt) => {
            let body = cfg.new_basic_block("body".to_string());
            let cond = cfg.new_basic_block("cond".to_string());
            let next = cfg.new_basic_block("next".to_string());
            let end = cfg.new_basic_block("endfor".to_string());

            vartab.new_scope();

            if let Some(init_stmt) = init_stmt {
                statement(init_stmt, f, cfg, ns, vartab, loops, errors)?;
            }

            cfg.add(vartab, Instr::Branch{ bb: cond });

            cfg.set_basic_block(cond);

            let (expr, expr_ty) = expression(cond_expr, cfg, ns, vartab, errors)?;

            cfg.add(vartab, Instr::BranchCond{
                cond: implicit_cast(&cond_expr.loc(), expr, &expr_ty, &resolver::TypeName::new_bool(), ns, errors)?,
                true_: body,
                false_: end,
            });

            cfg.set_basic_block(body);

            // continue goes to next, and if that does exist, cond
            loops.new_scope(end, match next_stmt { Some(_) => next, None => cond});
            vartab.new_dirty_tracker();

            let mut body_reachable = match body_stmt {
                Some(body_stmt) => statement(body_stmt, f, cfg, ns, vartab, loops, errors)?,
                None => true
            };

            if body_reachable {
                cfg.add(vartab, Instr::Branch{ bb: next });
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
                    cfg.add(vartab, Instr::Branch{ bb: cond });
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
        },
        _ => {
            panic!("not implemented")
        }
    }
}

fn coerce(l: &resolver::TypeName, l_loc: &ast::Loc, r: &resolver::TypeName, r_loc: &ast::Loc, ns: &resolver::ContractNameSpace, errors: &mut Vec<output::Output>) -> Result<resolver::TypeName, ()> {
    if *l == *r {
        return Ok(l.clone());
    }

    coerce_int(l, l_loc, r, r_loc, ns, errors)
}

fn get_int_length(l: &resolver::TypeName, l_loc: &ast::Loc, ns: &resolver::ContractNameSpace, errors: &mut Vec<output::Output>) -> Result<(u16, bool), ()> {
    Ok(match l {
        resolver::TypeName::Elementary(ast::ElementaryTypeName::Uint(n)) => (*n, false),
        resolver::TypeName::Elementary(ast::ElementaryTypeName::Int(n)) => (*n, true),
        resolver::TypeName::Elementary(t) => {
            errors.push(Output::error(*l_loc, format!("expression of type {} not allowed", t.to_string())));
            return Err(());
        },
        resolver::TypeName::Enum(n) => {
            errors.push(Output::error(*l_loc, format!("type enum {} not allowed", ns.enums[*n].name)));
            return Err(());
        }
    })
}

fn coerce_int(l: &resolver::TypeName, l_loc: &ast::Loc, r: &resolver::TypeName, r_loc: &ast::Loc, ns: &resolver::ContractNameSpace, errors: &mut Vec<output::Output>) -> Result<resolver::TypeName, ()> {
    let (left_len, left_signed) = get_int_length(l, l_loc, ns, errors)?;

    let (right_len, right_signed) = get_int_length(r, r_loc, ns, errors)?;

    Ok(resolver::TypeName::Elementary(match (left_signed, right_signed) {
        (true, true) => ast::ElementaryTypeName::Int(cmp::max(left_len, right_len)),
        (false, false) => ast::ElementaryTypeName::Uint(cmp::max(left_len, right_len)),
        (true, false) => {
            ast::ElementaryTypeName::Int(cmp::max(left_len, cmp::min(right_len + 8, 256)))
        },
        (false, true) => {
            ast::ElementaryTypeName::Int(cmp::max(cmp::min(left_len + 8, 256), right_len))
        }
    }))
}

fn implicit_cast(loc: &ast::Loc, expr: Expression, from: &resolver::TypeName, to: &resolver::TypeName, ns: &resolver::ContractNameSpace, errors: &mut Vec<output::Output>) -> Result<Expression, ()> {
    if from == to {
        return Ok(expr)
    }

    match (from, to) {
        (resolver::TypeName::Elementary(ast::ElementaryTypeName::Uint(from_len)),
         resolver::TypeName::Elementary(ast::ElementaryTypeName::Uint(to_len))) |
        (resolver::TypeName::Elementary(ast::ElementaryTypeName::Int(from_len)),
         resolver::TypeName::Elementary(ast::ElementaryTypeName::Uint(to_len))) => {
            if from_len > to_len {
                errors.push(Output::error(*loc, format!("implicit conversion would truncate from {} to {}", from.to_string(ns), to.to_string(ns))));
                return Err(());
            }

            if from_len < to_len {
                Ok(Expression::ZeroExt(to.clone(), Box::new(expr)))
            } else {
                Ok(expr)
            }
        },
        (resolver::TypeName::Elementary(ast::ElementaryTypeName::Int(from_len)),
         resolver::TypeName::Elementary(ast::ElementaryTypeName::Int(to_len))) |
        (resolver::TypeName::Elementary(ast::ElementaryTypeName::Uint(from_len)),
         resolver::TypeName::Elementary(ast::ElementaryTypeName::Int(to_len))) => {
            if from_len > to_len {
                errors.push(Output::error(*loc, format!("implicit conversion would truncate from {} to {}", from.to_string(ns), to.to_string(ns))));
                return Err(());
            }

            if from_len < to_len {
                Ok(Expression::SignExt(to.clone(), Box::new(expr)))
            } else {
                Ok(expr)
            }
        },
        (resolver::TypeName::Elementary(ast::ElementaryTypeName::Bytes(from_len)),
         resolver::TypeName::Elementary(ast::ElementaryTypeName::Bytes(to_len))) => {
            if from_len > to_len {
                errors.push(Output::error(*loc, format!("implicit conversion would truncate from {} to {}", from.to_string(ns), to.to_string(ns))));
                return Err(());
            }

            Ok(expr)
        },
        (resolver::TypeName::Elementary(ast::ElementaryTypeName::Bytes(_)),
         resolver::TypeName::Elementary(ast::ElementaryTypeName::String)) => {
            Ok(expr)
        },
        (resolver::TypeName::Elementary(ast::ElementaryTypeName::String),
         resolver::TypeName::Elementary(ast::ElementaryTypeName::Bytes(to_len))) => {
            match &expr {
                Expression::StringLiteral(from_str) => {
                    if from_str.len() > *to_len as usize {
                        errors.push(Output::error(*loc, format!("string of {} bytes is too long to fit into {}", from_str.len(), to.to_string(ns))));
                        return Err(())
                    }
                },
                _ => ()
            }

            Ok(expr)
        },
        _ => {
             errors.push(Output::error(*loc, format!("implicit conversion from {} to {} not possible", from.to_string(ns), to.to_string(ns))));
            Err(())
        }
    }
}

fn expression(expr: &ast::Expression, cfg: &mut ControlFlowGraph, ns: &resolver::ContractNameSpace, vartab: &mut Vartable, errors: &mut Vec<output::Output>) -> Result<(Expression, resolver::TypeName), ()> {
    match expr {
        ast::Expression::BoolLiteral(_, v) => {
            Ok((Expression::BoolLiteral(*v), resolver::TypeName::Elementary(ast::ElementaryTypeName::Bool)))
        },
        ast::Expression::StringLiteral(loc, v) => {
            // unescape supports octal escape values, solc does not
            // neither solc nor unescape support unicode code points like \u{61}
            match unescape(v) {
                Some(v) => {
                    Ok((Expression::StringLiteral(v), resolver::TypeName::Elementary(ast::ElementaryTypeName::String)))
                },
                None => {
                    // would be helpful if unescape told us what/where the problem was
                    errors.push(Output::error(loc.clone(), format!("string \"{}\" has invalid escape", v)));
                    Err(())
                }
            }
        },
        ast::Expression::HexLiteral(loc, v) => {
            if (v.len() % 2) != 0 {
                errors.push(Output::error(loc.clone(), format!("hex string \"{}\" has odd number of characters", v)));
                Err(())
            } else {
                let bs = hex::decode(v).unwrap();
                let len = bs.len() as u8;
                Ok((Expression::HexLiteral(bs), resolver::TypeName::Elementary(ast::ElementaryTypeName::Bytes(len))))
            }
        },
        ast::Expression::NumberLiteral(loc, b) => {
            // Return smallest type
            let bits = b.bits();

            let int_size = if bits < 7 {
                8
            } else {
                (bits + 7) & !7
            } as u16;

            if b.sign() == Sign::Minus {
                if bits > 255 {
                    errors.push(Output::error(loc.clone(), format!("{} is too large", b)));
                    Err(())
                } else {
                    Ok((Expression::NumberLiteral(int_size, b.clone()), resolver::TypeName::Elementary(ast::ElementaryTypeName::Int(int_size))))
                }
            } else {
                if bits > 256 {
                    errors.push(Output::error(loc.clone(), format!("{} is too large", b)));
                    Err(())
                } else {
                    Ok((Expression::NumberLiteral(int_size, b.clone()), resolver::TypeName::Elementary(ast::ElementaryTypeName::Uint(int_size))))
                }
            }
        },
        ast::Expression::Variable(id) => {
            match vartab.find(&id.name) {
                Some(v) => {
                    Ok((Expression::Variable(id.loc, v.pos), v.ty.clone()))
                },
                None => {
                    errors.push(Output::error(id.loc, format!("undeclared identifier {}", id.name.to_string())));
                    Err(())
                }
            }
        },
        ast::Expression::Add(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce_int(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((Expression::Add(
                Box::new(implicit_cast(&l.loc(), left, &left_type, &ty, ns, errors)?),
                Box::new(implicit_cast(&r.loc(), right, &right_type, &ty, ns, errors)?)),
                ty))
        },
        ast::Expression::Subtract(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce_int(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((Expression::Subtract(
                Box::new(implicit_cast(&l.loc(), left, &left_type, &ty, ns, errors)?),
                Box::new(implicit_cast(&r.loc(), right, &right_type, &ty, ns, errors)?)),
                ty))
        },
        ast::Expression::Multiply(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce_int(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((Expression::Multiply(
                Box::new(implicit_cast(&l.loc(), left, &left_type, &ty, ns, errors)?),
                Box::new(implicit_cast(&r.loc(), right, &right_type, &ty, ns, errors)?)),
                ty))
        },
        ast::Expression::Divide(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce_int(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            if ty.signed() {
                Ok((Expression::SDivide(
                    Box::new(implicit_cast(&l.loc(), left, &left_type, &ty, ns, errors)?),
                    Box::new(implicit_cast(&r.loc(), right, &right_type, &ty, ns, errors)?)),
                    ty))
            } else {
                Ok((Expression::UDivide(
                    Box::new(implicit_cast(&l.loc(), left, &left_type, &ty, ns, errors)?),
                    Box::new(implicit_cast(&r.loc(), right, &right_type, &ty, ns, errors)?)),
                    ty))
            }
        },
        ast::Expression::Modulo(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce_int(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            if ty.signed() {
                Ok((Expression::SModulo(
                    Box::new(implicit_cast(&l.loc(), left, &left_type, &ty, ns, errors)?),
                    Box::new(implicit_cast(&r.loc(), right, &right_type, &ty, ns, errors)?)),
                    ty))
            } else {
                Ok((Expression::UModulo(
                    Box::new(implicit_cast(&l.loc(), left, &left_type, &ty, ns, errors)?),
                    Box::new(implicit_cast(&r.loc(), right, &right_type, &ty, ns, errors)?)),
                    ty))
            }
        },

        // compare
        ast::Expression::More(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce_int(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((Expression::More(
                Box::new(implicit_cast(&l.loc(), left, &left_type, &ty, ns, errors)?),
                Box::new(implicit_cast(&r.loc(), right, &right_type, &ty, ns, errors)?)),
                resolver::TypeName::new_bool()))
        },
        ast::Expression::Less(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce_int(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((Expression::Less(
                Box::new(implicit_cast(&l.loc(), left, &left_type, &ty, ns, errors)?),
                Box::new(implicit_cast(&r.loc(), right, &right_type, &ty, ns, errors)?)),
                resolver::TypeName::new_bool()))
        },
        ast::Expression::MoreEqual(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce_int(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((Expression::MoreEqual(
                Box::new(implicit_cast(&l.loc(), left, &left_type, &ty, ns, errors)?),
                Box::new(implicit_cast(&r.loc(), right, &right_type, &ty, ns, errors)?)),
                resolver::TypeName::new_bool()))
        },
        ast::Expression::LessEqual(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce_int(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((Expression::LessEqual(
                Box::new(implicit_cast(&l.loc(), left, &left_type, &ty, ns, errors)?),
                Box::new(implicit_cast(&r.loc(), right, &right_type, &ty, ns, errors)?)),
                resolver::TypeName::new_bool()))
        },
        ast::Expression::Equal(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((Expression::Equal(
                Box::new(implicit_cast(&l.loc(), left, &left_type, &ty, ns, errors)?),
                Box::new(implicit_cast(&r.loc(), right, &right_type, &ty, ns, errors)?)),
                resolver::TypeName::new_bool()))
        },
        ast::Expression::NotEqual(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((Expression::NotEqual(
                Box::new(implicit_cast(&l.loc(), left, &left_type, &ty, ns, errors)?),
                Box::new(implicit_cast(&r.loc(), right, &right_type, &ty, ns, errors)?)),
                resolver::TypeName::new_bool()))
        },

        // unary expressions
        ast::Expression::Not(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, ns, vartab, errors)?;

            Ok((Expression::Not(
                Box::new(implicit_cast(&loc, expr, &expr_type, &resolver::TypeName::new_bool(), ns, errors)?)),
                resolver::TypeName::new_bool()))
        },
        ast::Expression::Complement(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, ns, vartab, errors)?;

            get_int_length(&expr_type, loc, ns, errors)?;

            Ok((Expression::Complement(Box::new(expr)), expr_type))
        },
        ast::Expression::UnaryMinus(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, ns, vartab, errors)?;

            get_int_length(&expr_type, loc, ns, errors)?;

            Ok((Expression::UnaryMinus(Box::new(expr)), expr_type))
        },
        ast::Expression::UnaryPlus(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, ns, vartab, errors)?;

            get_int_length(&expr_type, loc, ns, errors)?;

            Ok((expr, expr_type))
        },

        // pre/post decrement/increment
        ast::Expression::PostIncrement(loc, var) |
        ast::Expression::PreIncrement(loc, var) |
        ast::Expression::PostDecrement(loc, var) |
        ast::Expression::PreDecrement(loc, var) => {
            let id = match var.as_ref() {
                ast::Expression::Variable(id) => id,
                _ => unreachable!()
            };

            let (pos, ty) = match vartab.find(&id.name) {
                Some(v) => {
                    (v.pos, v.ty.clone())
                },
                None => {
                    errors.push(Output::error(id.loc, format!("undeclared identifier {}", id.name.to_string())));
                    return Err(());
                }
            };

            get_int_length(&ty, loc, ns, errors)?;

            match expr {
                ast::Expression::PostIncrement(_, _) => {
                    let temp_pos = vartab.temp(id, &ty);
                    cfg.add(vartab, Instr::Set{
                        res: temp_pos,
                        expr: Expression::Variable(id.loc.clone(), pos),
                    });
                    cfg.add(vartab, Instr::Set{
                        res: pos,
                        expr: Expression::Add(
                            Box::new(Expression::Variable(id.loc.clone(), pos)),
                            Box::new(Expression::NumberLiteral(ty.bits(), One::one())))
                    });

                    Ok((Expression::Variable(id.loc.clone(), temp_pos), ty))
                },
                ast::Expression::PostDecrement(_, _) => {
                    let temp_pos = vartab.temp(id, &ty);
                    cfg.add(vartab, Instr::Set{
                        res: temp_pos,
                        expr: Expression::Variable(id.loc.clone(), pos),
                    });
                    cfg.add(vartab, Instr::Set{
                        res: pos,
                        expr: Expression::Subtract(
                            Box::new(Expression::Variable(id.loc.clone(), pos)),
                            Box::new(Expression::NumberLiteral(ty.bits(), One::one())))
                    });

                    Ok((Expression::Variable(id.loc.clone(), temp_pos), ty))
                },
                ast::Expression::PreIncrement(_, _) => {
                    let temp_pos = vartab.temp(id, &ty);
                    cfg.add(vartab, Instr::Set{
                        res: pos,
                        expr: Expression::Subtract(
                            Box::new(Expression::Variable(id.loc.clone(), pos)),
                            Box::new(Expression::NumberLiteral(ty.bits(), One::one())))
                    });
                    cfg.add(vartab, Instr::Set{
                        res: temp_pos,
                        expr: Expression::Variable(id.loc.clone(), pos),
                    });

                    Ok((Expression::Variable(id.loc.clone(), temp_pos), ty))
                },
                ast::Expression::PreDecrement(_, _) => {
                    let temp_pos = vartab.temp(id, &ty);
                    cfg.add(vartab, Instr::Set{
                        res: pos,
                        expr: Expression::Subtract(
                            Box::new(Expression::Variable(id.loc.clone(), pos)),
                            Box::new(Expression::NumberLiteral(ty.bits(), One::one())))
                    });
                    cfg.add(vartab, Instr::Set{
                        res: temp_pos,
                        expr: Expression::Variable(id.loc.clone(), pos),
                    });

                    Ok((Expression::Variable(id.loc.clone(), temp_pos), ty))
                },
                _ => unreachable!()
            }
        },

        // assignment
        ast::Expression::Assign(_, var, e) => {
            let id = match var.as_ref() {
                ast::Expression::Variable(id) => id,
                _ => unreachable!()
            };

            let (expr, expr_type) = expression(e, cfg, ns, vartab, errors)?;

            let (res, ty) = match vartab.find(&id.name) {
                Some(v) => {
                    (v.pos, v.ty.clone())
                },
                None => {
                    errors.push(Output::error(id.loc, format!("undeclared identifier {}", id.name.to_string())));
                    return Err(());
                }
            };

            cfg.add(vartab, Instr::Set{
                res,
                expr: implicit_cast(&id.loc, expr, &expr_type, &ty, ns, errors)?,
            });

            Ok((Expression::Variable(id.loc.clone(), res), ty))
        },

        ast::Expression::AssignAdd(_, var, e) |
        ast::Expression::AssignSubtract(_, var, e) |
        ast::Expression::AssignMultiply(_, var, e) |
        ast::Expression::AssignDivide(_, var, e) |
        ast::Expression::AssignModulo(_, var, e) => {
            let id = match var.as_ref() {
                ast::Expression::Variable(id) => id,
                _ => unreachable!()
            };

            let (pos, ty) = match vartab.find(&id.name) {
                Some(v) => {
                    (v.pos, v.ty.clone())
                },
                None => {
                    errors.push(Output::error(id.loc, format!("undeclared identifier {}", id.name.to_string())));
                    return Err(());
                }
            };

            if !ty.ordered() {
                errors.push(Output::error(id.loc, format!("variable {} not ordered", id.name.to_string())));
                return Err(());
            }

            let (set, set_type) = expression(e, cfg, ns, vartab, errors)?;

            let set = implicit_cast(&id.loc, set, &set_type, &ty, ns, errors)?;

            let set = match expr {
                ast::Expression::AssignAdd(_, _, _) => {
                    Expression::Add(Box::new(Expression::Variable(id.loc, pos)), Box::new(set))
                },
                ast::Expression::AssignSubtract(_, _, _) => {
                    Expression::Subtract(Box::new(Expression::Variable(id.loc, pos)), Box::new(set))
                },
                ast::Expression::AssignMultiply(_, _, _) => {
                    Expression::Multiply(Box::new(Expression::Variable(id.loc, pos)), Box::new(set))
                },
                ast::Expression::AssignDivide(_, _, _) => {
                    if ty.signed() {
                        Expression::SDivide(Box::new(Expression::Variable(id.loc, pos)), Box::new(set))
                    } else {
                        Expression::UDivide(Box::new(Expression::Variable(id.loc, pos)), Box::new(set))
                    }
                },
                ast::Expression::AssignModulo(_, _, _) => {
                    if ty.signed() {
                        Expression::SModulo(Box::new(Expression::Variable(id.loc, pos)), Box::new(set))
                    } else {
                        Expression::UModulo(Box::new(Expression::Variable(id.loc, pos)), Box::new(set))
                    }
                },
                _ => unreachable!()
            };

            cfg.add(vartab, Instr::Set{
                res: pos,
                expr: set,
            });

            Ok((Expression::Variable(id.loc.clone(), pos), ty))
        }
        _ => unimplemented!()
    }
}

// Vartable
// methods
// create variable with loc, name, TypeName -> pos
// find variable by name -> Some(pos)
// new scope
// leave scope
// produce full Vector of all variables

pub struct Variable {
    pub id: ast::Identifier,
    pub ty: resolver::TypeName,
    pub pos: usize,
}

struct VarScope (
    HashMap<String, usize>,
    Option<HashSet<usize>>
);

pub struct Vartable {
    vars: Vec<Variable>,
    names: LinkedList<VarScope>,
    dirty: Vec<DirtyTracker>,
}

pub struct DirtyTracker {
    lim: usize,
    set: HashSet<usize>,
}

impl Vartable {
    pub fn new() -> Self {
        let mut list = LinkedList::new();
        list.push_front(VarScope(HashMap::new(), None));
        Vartable{vars: Vec::new(), names: list, dirty: Vec::new()}
    }

    pub fn add(&mut self, id: &ast::Identifier, ty: resolver::TypeName, errors: &mut Vec<output::Output>) -> Option<usize> {
        if let Some(ref prev) = self.find(&id.name) {
            errors.push(Output::error_with_note(id.loc, format!("{} is already declared", id.name.to_string()),
                    prev.id.loc.clone(), "location of previous declaration".to_string()));
            return None;
        }

        let pos = self.vars.len();

        self.vars.push(Variable{
            id: id.clone(),
            ty,
            pos,
        });

        self.names.front_mut().unwrap().0.insert(id.name.to_string(), pos);

        Some(pos)
    }

    pub fn find(&self, name: &String) -> Option<&Variable> {
        for scope in &self.names {
            if let Some(n) = scope.0.get(name) {
                return Some(&self.vars[*n]);
            }
        }

        None
    }

    pub fn temp(&mut self, id: &ast::Identifier, ty: &resolver::TypeName) -> usize {
        let pos = self.vars.len();

        self.vars.push(Variable{
            id: ast::Identifier{
                name: format!("{}.temp.{}", id.name, pos),
                loc: id.loc,
            },
            ty: ty.clone(),
            pos,
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
        self.dirty.push(DirtyTracker{
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
    no_continues: usize
}

struct LoopScopes(LinkedList<LoopScope>);

impl LoopScopes {
    fn new() -> Self {
        LoopScopes(LinkedList::new())
    }

    fn new_scope(&mut self, break_bb: usize, continue_bb: usize) {
        self.0.push_front(LoopScope{
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
            },
            None => None
        }
    }

    fn do_continue(&mut self) -> Option<usize> {
        match self.0.front_mut() {
            Some(scope) => {
                scope.no_continues += 1;
                Some(scope.continue_bb)
            },
            None => None
        }
    }
}
