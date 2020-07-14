use num_bigint::BigInt;
use std::collections::LinkedList;

use super::cfg::{ControlFlowGraph, Instr, Vartable};
use super::expression::{assign_single, emit_function_call, expression};
use parser::pt;
use sema::ast::{
    CallTy, DestructureField, Expression, Function, Namespace, Parameter, Statement, Type,
};
use sema::expression::try_cast;

/// Resolve a statement, which might be a block of statements or an entire body of a function
pub fn statement(
    stmt: &Statement,
    f: &Function,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    loops: &mut LoopScopes,
) {
    match stmt {
        Statement::VariableDecl(_, pos, _, Some(init)) => {
            let expr = expression(init, cfg, contract_no, ns, vartab);

            cfg.add(vartab, Instr::Set { res: *pos, expr });
        }
        Statement::VariableDecl(_, _, _, None) => {
            // nothing to do
        }
        Statement::Return(_, values) => {
            let values = values
                .iter()
                .map(|expr| expression(expr, cfg, contract_no, ns, vartab))
                .collect();

            cfg.add(vartab, Instr::Return { value: values });
        }
        Statement::Expression(_, reachable, expr) => {
            let expr = expression(expr, cfg, contract_no, ns, vartab);

            if expr != Expression::Poison {
                cfg.add(vartab, Instr::Eval { expr });
            }

            if !reachable {
                cfg.add(vartab, Instr::Unreachable);
            }
        }
        Statement::Delete(_, ty, expr) => {
            let var_expr = expression(expr, cfg, contract_no, ns, vartab);

            cfg.add(
                vartab,
                Instr::ClearStorage {
                    ty: ty.clone(),
                    storage: var_expr,
                },
            );
        }
        Statement::Break(_) => {
            cfg.add(
                vartab,
                Instr::Branch {
                    bb: loops.do_break(),
                },
            );
        }
        Statement::Continue(_) => {
            cfg.add(
                vartab,
                Instr::Branch {
                    bb: loops.do_continue(),
                },
            );
        }
        Statement::If(_, _, cond, then_stmt, else_stmt) if else_stmt.is_empty() => {
            if_then(cond, then_stmt, f, cfg, contract_no, ns, vartab, loops);
        }
        Statement::If(_, _, cond, then_stmt, else_stmt) => if_then_else(
            cond,
            then_stmt,
            else_stmt,
            f,
            cfg,
            contract_no,
            ns,
            vartab,
            loops,
        ),
        Statement::DoWhile(_, _, body_stmt, cond_expr) => {
            let body = cfg.new_basic_block("body".to_string());
            let cond = cfg.new_basic_block("conf".to_string());
            let end = cfg.new_basic_block("enddowhile".to_string());

            cfg.add(vartab, Instr::Branch { bb: body });

            cfg.set_basic_block(body);

            vartab.new_dirty_tracker();
            loops.new_scope(end, cond);

            let mut body_reachable = true;

            for stmt in body_stmt {
                statement(stmt, f, cfg, contract_no, ns, vartab, loops);

                body_reachable = stmt.reachable();
            }

            if body_reachable {
                cfg.add(vartab, Instr::Branch { bb: cond });
            }

            cfg.set_basic_block(cond);

            let cond_expr = expression(cond_expr, cfg, contract_no, ns, vartab);

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: cond_expr,
                    true_: body,
                    false_: end,
                },
            );

            let set = vartab.pop_dirty_tracker();
            cfg.set_phis(end, set.clone());
            cfg.set_phis(body, set.clone());
            cfg.set_phis(cond, set);

            cfg.set_basic_block(end);
        }
        Statement::While(_, _, cond_expr, body_stmt) => {
            let cond = cfg.new_basic_block("cond".to_string());
            let body = cfg.new_basic_block("body".to_string());
            let end = cfg.new_basic_block("endwhile".to_string());

            cfg.add(vartab, Instr::Branch { bb: cond });

            cfg.set_basic_block(cond);

            let cond_expr = expression(cond_expr, cfg, contract_no, ns, vartab);

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: cond_expr,
                    true_: body,
                    false_: end,
                },
            );

            cfg.set_basic_block(body);

            vartab.new_dirty_tracker();
            loops.new_scope(end, cond);

            let mut body_reachable = true;

            for stmt in body_stmt {
                statement(stmt, f, cfg, contract_no, ns, vartab, loops);

                body_reachable = stmt.reachable();
            }

            if body_reachable {
                cfg.add(vartab, Instr::Branch { bb: cond });
            }

            loops.leave_scope();
            let set = vartab.pop_dirty_tracker();
            cfg.set_phis(end, set.clone());
            cfg.set_phis(cond, set);

            cfg.set_basic_block(end);
        }
        Statement::For {
            init,
            cond: None,
            next,
            body,
            ..
        } => {
            let body_block = cfg.new_basic_block("body".to_string());
            let next_block = cfg.new_basic_block("next".to_string());
            let end_block = cfg.new_basic_block("endfor".to_string());

            for stmt in init {
                statement(stmt, f, cfg, contract_no, ns, vartab, loops);
            }

            cfg.add(vartab, Instr::Branch { bb: body_block });

            cfg.set_basic_block(body_block);

            loops.new_scope(
                end_block,
                if next.is_empty() {
                    body_block
                } else {
                    next_block
                },
            );

            vartab.new_dirty_tracker();

            let mut body_reachable = true;

            for stmt in body {
                statement(stmt, f, cfg, contract_no, ns, vartab, loops);

                body_reachable = stmt.reachable();
            }

            if body_reachable {
                cfg.add(vartab, Instr::Branch { bb: next_block });
            }

            loops.leave_scope();

            if body_reachable {
                if !next.is_empty() {
                    cfg.set_basic_block(next_block);

                    for stmt in next {
                        statement(stmt, f, cfg, contract_no, ns, vartab, loops);
                        body_reachable = stmt.reachable();
                    }
                }

                if body_reachable {
                    cfg.add(vartab, Instr::Branch { bb: body_block });
                }
            }

            let set = vartab.pop_dirty_tracker();

            cfg.set_phis(next_block, set.clone());
            cfg.set_phis(body_block, set.clone());
            cfg.set_phis(end_block, set);

            cfg.set_basic_block(end_block);
        }
        Statement::For {
            init,
            cond: Some(cond_expr),
            next,
            body,
            ..
        } => {
            let body_block = cfg.new_basic_block("body".to_string());
            let cond_block = cfg.new_basic_block("cond".to_string());
            let next_block = cfg.new_basic_block("next".to_string());
            let end_block = cfg.new_basic_block("endfor".to_string());

            for stmt in init {
                statement(stmt, f, cfg, contract_no, ns, vartab, loops);
            }

            cfg.add(vartab, Instr::Branch { bb: cond_block });

            cfg.set_basic_block(cond_block);

            let cond_expr = expression(cond_expr, cfg, contract_no, ns, vartab);

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: cond_expr,
                    true_: body_block,
                    false_: end_block,
                },
            );

            cfg.set_basic_block(body_block);

            // continue goes to next
            loops.new_scope(end_block, next_block);

            vartab.new_dirty_tracker();

            let mut body_reachable = true;

            for stmt in body {
                statement(stmt, f, cfg, contract_no, ns, vartab, loops);

                body_reachable = stmt.reachable();
            }

            if body_reachable {
                cfg.add(vartab, Instr::Branch { bb: next_block });
            }

            loops.leave_scope();

            cfg.set_basic_block(next_block);

            let mut next_reachable = true;

            for stmt in next {
                statement(stmt, f, cfg, contract_no, ns, vartab, loops);

                next_reachable = stmt.reachable();
            }

            if next_reachable {
                cfg.add(vartab, Instr::Branch { bb: cond_block });
            }

            cfg.set_basic_block(end_block);

            let set = vartab.pop_dirty_tracker();
            cfg.set_phis(next_block, set.clone());
            cfg.set_phis(end_block, set.clone());
            cfg.set_phis(cond_block, set);
        }
        Statement::Destructure(_, fields, expr) => {
            let mut values = match expr {
                Expression::List(_, exprs) => {
                    let mut values = Vec::new();

                    for expr in exprs {
                        let loc = expr.loc();
                        let expr = expression(expr, cfg, contract_no, ns, vartab);
                        let ty = expr.ty();

                        let res = vartab.temp_anonymous(&ty);

                        cfg.add(vartab, Instr::Set { res, expr });

                        values.push(Expression::Variable(loc, ty, res));
                    }

                    values
                }
                _ => {
                    // must be function call, either internal or external
                    emit_function_call(expr, contract_no, cfg, ns, vartab)
                }
            };

            for field in fields.iter() {
                let right = values.remove(0);

                match field {
                    DestructureField::None => {
                        // nothing to do
                    }
                    DestructureField::VariableDecl(res, param) => {
                        // the resolver did not cast the expression
                        let expr = try_cast(&param.loc, right, &param.ty, true, ns)
                            .expect("sema should have checked cast");

                        cfg.add(vartab, Instr::Set { res: *res, expr });
                    }
                    DestructureField::Expression(left) => {
                        // the resolver did not cast the expression
                        let loc = left.loc();

                        let expr = try_cast(&loc, right, left.ty().deref_any(), true, ns)
                            .expect("sema should have checked cast");

                        assign_single(left, &expr, cfg, contract_no, ns, vartab);
                    }
                }
            }
        }
        Statement::TryCatch {
            expr,
            returns,
            ok_stmt,
            error,
            catch_param_pos,
            catch_stmt,
            ..
        } => try_catch(
            expr,
            returns,
            ok_stmt,
            error,
            catch_param_pos,
            catch_stmt,
            f,
            cfg,
            contract_no,
            ns,
            vartab,
            loops,
        ),
    }
}

/// Generate if-then-no-else
fn if_then(
    cond: &Expression,
    then_stmt: &[Statement],
    f: &Function,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    loops: &mut LoopScopes,
) {
    let cond = expression(cond, cfg, contract_no, ns, vartab);

    let then = cfg.new_basic_block("then".to_string());
    let endif = cfg.new_basic_block("endif".to_string());

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_: then,
            false_: endif,
        },
    );

    cfg.set_basic_block(then);

    vartab.new_dirty_tracker();

    let mut reachable = true;

    for stmt in then_stmt {
        statement(stmt, f, cfg, contract_no, ns, vartab, loops);

        reachable = stmt.reachable();
    }

    if reachable {
        cfg.add(vartab, Instr::Branch { bb: endif });
    }

    cfg.set_phis(endif, vartab.pop_dirty_tracker());

    cfg.set_basic_block(endif);
}

/// Generate if-then-else
fn if_then_else(
    cond: &Expression,
    then_stmt: &[Statement],
    else_stmt: &[Statement],
    f: &Function,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    loops: &mut LoopScopes,
) {
    let cond = expression(cond, cfg, contract_no, ns, vartab);

    let then = cfg.new_basic_block("then".to_string());
    let else_ = cfg.new_basic_block("else".to_string());
    let endif = cfg.new_basic_block("endif".to_string());

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_: then,
            false_: else_,
        },
    );

    // then
    cfg.set_basic_block(then);

    vartab.new_dirty_tracker();

    let mut then_reachable = true;

    for stmt in then_stmt {
        statement(stmt, f, cfg, contract_no, ns, vartab, loops);

        then_reachable = stmt.reachable();
    }

    if then_reachable {
        cfg.add(vartab, Instr::Branch { bb: endif });
    }

    // else
    cfg.set_basic_block(else_);

    let mut else_reachable = true;

    for stmt in else_stmt {
        statement(stmt, f, cfg, contract_no, ns, vartab, loops);

        else_reachable = stmt.reachable();
    }

    if else_reachable {
        cfg.add(vartab, Instr::Branch { bb: endif });
    }

    cfg.set_phis(endif, vartab.pop_dirty_tracker());

    cfg.set_basic_block(endif);
}

/// Resolve try catch statement
#[allow(clippy::too_many_arguments)]
fn try_catch(
    fcall: &Expression,
    returns: &[(Option<usize>, Parameter)],
    ok_stmt: &[Statement],
    error: &Option<(Option<usize>, Parameter, Vec<Statement>)>,
    catch_param_pos: &Option<usize>,
    catch_stmt: &[Statement],
    f: &Function,
    cfg: &mut ControlFlowGraph,
    callee_contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    loops: &mut LoopScopes,
) {
    let success = vartab.temp(
        &pt::Identifier {
            loc: fcall.loc(),
            name: "success".to_owned(),
        },
        &Type::Bool,
    );

    let success_block = cfg.new_basic_block("success".to_string());
    let catch_block = cfg.new_basic_block("catch".to_string());
    let finally_block = cfg.new_basic_block("finally".to_string());

    match &fcall {
        Expression::ExternalFunctionCall {
            contract_no,
            function_no,
            address,
            args,
            value,
            gas,
            ..
        } => {
            let address = expression(address, cfg, callee_contract_no, ns, vartab);
            let value = expression(value, cfg, callee_contract_no, ns, vartab);
            let gas = expression(gas, cfg, callee_contract_no, ns, vartab);

            let args = args
                .iter()
                .map(|a| expression(a, cfg, callee_contract_no, ns, vartab))
                .collect();

            cfg.add(
                vartab,
                Instr::ExternalCall {
                    success: Some(success),
                    address,
                    contract_no: Some(*contract_no),
                    function_no: *function_no,
                    args,
                    value,
                    gas,
                    callty: CallTy::Regular,
                },
            );

            let ftype = &ns.contracts[*contract_no].functions[*function_no];

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: Expression::Variable(fcall.loc(), Type::Bool, success),
                    true_: success_block,
                    false_: catch_block,
                },
            );

            cfg.set_basic_block(success_block);

            if !ftype.returns.is_empty() {
                let mut res = Vec::new();

                for ret in returns {
                    res.push(match ret {
                        (Some(pos), _) => *pos,
                        (None, param) => vartab.temp_anonymous(&param.ty),
                    });
                }

                cfg.add(
                    vartab,
                    Instr::AbiDecode {
                        res,
                        selector: None,
                        exception: None,
                        tys: ftype.returns.clone(),
                        data: Expression::ReturnData(pt::Loc(0, 0, 0)),
                    },
                );
            }
        }
        Expression::Constructor {
            contract_no,
            constructor_no,
            args,
            value,
            gas,
            salt,
            ..
        } => {
            let address_res = match returns.get(0) {
                Some((Some(pos), _)) => *pos,
                _ => vartab.temp_anonymous(&Type::Contract(*contract_no)),
            };

            let value = match value {
                Some(v) => Some(expression(v, cfg, callee_contract_no, ns, vartab)),
                None => None,
            };

            let gas = expression(gas, cfg, callee_contract_no, ns, vartab);
            let salt = salt
                .as_ref()
                .map(|gas| expression(gas, cfg, callee_contract_no, ns, vartab));

            let args = args
                .iter()
                .map(|a| expression(a, cfg, callee_contract_no, ns, vartab))
                .collect();

            cfg.add(
                vartab,
                Instr::Constructor {
                    success: Some(success),
                    res: address_res,
                    contract_no: *contract_no,
                    constructor_no: *constructor_no,
                    args,
                    value,
                    gas,
                    salt,
                },
            );

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: Expression::Variable(fcall.loc(), Type::Bool, success),
                    true_: success_block,
                    false_: catch_block,
                },
            );

            cfg.set_basic_block(success_block);
        }
        _ => unreachable!(),
    }

    vartab.new_dirty_tracker();

    let mut finally_reachable = true;

    for stmt in ok_stmt {
        statement(stmt, f, cfg, callee_contract_no, ns, vartab, loops);

        finally_reachable = stmt.reachable();
    }

    if finally_reachable {
        cfg.add(vartab, Instr::Branch { bb: finally_block });
    }

    cfg.set_basic_block(catch_block);

    if let Some((error_param_pos, error_param, error_stmt)) = error {
        let no_reason_block = cfg.new_basic_block("no_reason".to_string());

        let error_var = match error_param_pos {
            Some(pos) => *pos,
            _ => vartab.temp_anonymous(&Type::String),
        };

        cfg.add(
            vartab,
            Instr::AbiDecode {
                selector: Some(0x08c3_79a0),
                exception: Some(no_reason_block),
                res: vec![error_var],
                tys: vec![error_param.clone()],
                data: Expression::ReturnData(pt::Loc(0, 0, 0)),
            },
        );

        let mut reachable = true;

        for stmt in error_stmt {
            statement(stmt, f, cfg, callee_contract_no, ns, vartab, loops);

            reachable = stmt.reachable();
        }
        if reachable {
            cfg.add(vartab, Instr::Branch { bb: finally_block });
        }

        cfg.set_basic_block(no_reason_block);
    }

    if let Some(pos) = catch_param_pos {
        cfg.add(
            vartab,
            Instr::Set {
                res: *pos,
                expr: Expression::ReturnData(pt::Loc(0, 0, 0)),
            },
        );
    }

    let mut reachable = true;

    for stmt in catch_stmt {
        statement(stmt, f, cfg, callee_contract_no, ns, vartab, loops);

        reachable = stmt.reachable();
    }

    if reachable {
        cfg.add(vartab, Instr::Branch { bb: finally_block });
    }

    let set = vartab.pop_dirty_tracker();
    cfg.set_phis(finally_block, set);

    cfg.set_basic_block(finally_block);
}

pub struct LoopScope {
    break_bb: usize,
    continue_bb: usize,
}

pub struct LoopScopes(LinkedList<LoopScope>);

impl LoopScopes {
    pub fn new() -> Self {
        LoopScopes(LinkedList::new())
    }

    fn new_scope(&mut self, break_bb: usize, continue_bb: usize) {
        self.0.push_front(LoopScope {
            break_bb,
            continue_bb,
        })
    }

    fn leave_scope(&mut self) -> LoopScope {
        self.0.pop_front().expect("should be in loop scope")
    }

    fn do_break(&mut self) -> usize {
        self.0.front().unwrap().break_bb
    }

    fn do_continue(&mut self) -> usize {
        self.0.front().unwrap().continue_bb
    }
}

impl Type {
    pub fn default(&self, ns: &Namespace) -> Expression {
        match self {
            Type::Address(_) | Type::Uint(_) | Type::Int(_) => {
                Expression::NumberLiteral(pt::Loc(0, 0, 0), self.clone(), BigInt::from(0))
            }
            Type::Bool => Expression::BoolLiteral(pt::Loc(0, 0, 0), false),
            Type::Bytes(n) => {
                let mut l = Vec::new();
                l.resize(*n as usize, 0);
                Expression::BytesLiteral(pt::Loc(0, 0, 0), self.clone(), l)
            }
            Type::Enum(e) => ns.enums[*e].ty.default(ns),
            Type::Struct(_) => {
                Expression::StructLiteral(pt::Loc(0, 0, 0), self.clone(), Vec::new())
            }
            Type::Ref(_) => unreachable!(),
            Type::StorageRef(_) => Expression::Poison,
            Type::String | Type::DynamicBytes => {
                Expression::BytesLiteral(pt::Loc(0, 0, 0), self.clone(), vec![])
            }
            _ => unreachable!(),
        }
    }
}
