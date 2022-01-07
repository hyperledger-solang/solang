use num_bigint::BigInt;
use std::collections::LinkedList;

use super::expression::{assign_single, default_gas, emit_function_call, expression};
use super::Options;
use super::{
    cfg::{ControlFlowGraph, Instr},
    vartable::Vartable,
};
use crate::codegen::unused_variable::{
    should_remove_assignment, should_remove_variable, SideEffectsCheckParameters,
};
use crate::parser::pt;
use crate::sema::ast::{
    Builtin, CallTy, DestructureField, Expression, Function, Namespace, Parameter, Statement,
    TryCatch, Type,
};
use crate::sema::expression::cast;
use num_traits::Zero;
use tiny_keccak::{Hasher, Keccak};

/// Resolve a statement, which might be a block of statements or an entire body of a function
pub fn statement(
    stmt: &Statement,
    func: &Function,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    loops: &mut LoopScopes,
    placeholder: Option<&Instr>,
    return_override: Option<&Instr>,
    opt: &Options,
) {
    match stmt {
        Statement::Block { statements, .. } => {
            for stmt in statements {
                statement(
                    stmt,
                    func,
                    cfg,
                    contract_no,
                    ns,
                    vartab,
                    loops,
                    placeholder,
                    return_override,
                    opt,
                );
            }
        }
        Statement::VariableDecl(loc, pos, _, Some(init)) => {
            if should_remove_variable(pos, func, opt) {
                let mut params = SideEffectsCheckParameters {
                    cfg,
                    contract_no,
                    func: Some(func),
                    ns,
                    vartab,
                    opt,
                };

                //If we remove the assignment, we must keep expressions that have side effects
                init.recurse(&mut params, process_side_effects_expressions);
                return;
            }

            let expr = expression(init, cfg, contract_no, Some(func), ns, vartab, opt);
            cfg.add(
                vartab,
                Instr::Set {
                    loc: *loc,
                    res: *pos,
                    expr,
                },
            );
        }
        Statement::VariableDecl(loc, pos, param, None) => {
            if should_remove_variable(pos, func, opt) {
                return;
            }

            // Add variable as undefined
            cfg.add(
                vartab,
                Instr::Set {
                    loc: *loc,
                    res: *pos,
                    expr: Expression::Undefined(param.ty.clone()),
                },
            );
        }
        Statement::Return(_, expr) => {
            if let Some(return_instr) = return_override {
                cfg.add(vartab, return_instr.clone());
            } else {
                match expr {
                    None => cfg.add(vartab, Instr::Return { value: Vec::new() }),
                    Some(expr) => returns(expr, cfg, contract_no, func, ns, vartab, loops, opt),
                }
            }
        }
        Statement::Expression(_, reachable, expr) => {
            if let Expression::Assign(_, _, left, right) = &expr {
                if should_remove_assignment(ns, left, func, opt) {
                    let mut params = SideEffectsCheckParameters {
                        cfg,
                        contract_no,
                        func: Some(func),
                        ns,
                        vartab,
                        opt,
                    };
                    right.recurse(&mut params, process_side_effects_expressions);

                    if !reachable {
                        cfg.add(vartab, Instr::Unreachable);
                    }
                    return;
                }
            }

            let _ = expression(expr, cfg, contract_no, Some(func), ns, vartab, opt);

            if !reachable {
                cfg.add(vartab, Instr::Unreachable);
            }
        }
        Statement::Delete(_, ty, expr) => {
            let var_expr = expression(expr, cfg, contract_no, Some(func), ns, vartab, opt);

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
                    block: loops.do_break(),
                },
            );
        }
        Statement::Continue(_) => {
            cfg.add(
                vartab,
                Instr::Branch {
                    block: loops.do_continue(),
                },
            );
        }
        Statement::If(_, _, cond, then_stmt, else_stmt) if else_stmt.is_empty() => {
            if_then(
                cond,
                then_stmt,
                func,
                cfg,
                contract_no,
                ns,
                vartab,
                loops,
                placeholder,
                return_override,
                opt,
            );
        }
        Statement::If(_, _, cond, then_stmt, else_stmt) => if_then_else(
            cond,
            then_stmt,
            else_stmt,
            func,
            cfg,
            contract_no,
            ns,
            vartab,
            loops,
            placeholder,
            return_override,
            opt,
        ),
        Statement::DoWhile(_, _, body_stmt, cond_expr) => {
            let body = cfg.new_basic_block("body".to_string());
            let cond = cfg.new_basic_block("conf".to_string());
            let end = cfg.new_basic_block("enddowhile".to_string());

            cfg.add(vartab, Instr::Branch { block: body });

            cfg.set_basic_block(body);

            vartab.new_dirty_tracker(ns.next_id);
            loops.new_scope(end, cond);

            let mut body_reachable = true;

            for stmt in body_stmt {
                statement(
                    stmt,
                    func,
                    cfg,
                    contract_no,
                    ns,
                    vartab,
                    loops,
                    placeholder,
                    return_override,
                    opt,
                );

                body_reachable = stmt.reachable();
            }

            if body_reachable {
                cfg.add(vartab, Instr::Branch { block: cond });
            }

            cfg.set_basic_block(cond);

            let cond_expr = expression(cond_expr, cfg, contract_no, Some(func), ns, vartab, opt);

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: cond_expr,
                    true_block: body,
                    false_block: end,
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

            cfg.add(vartab, Instr::Branch { block: cond });

            cfg.set_basic_block(cond);

            let cond_expr = expression(cond_expr, cfg, contract_no, Some(func), ns, vartab, opt);

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: cond_expr,
                    true_block: body,
                    false_block: end,
                },
            );

            cfg.set_basic_block(body);

            vartab.new_dirty_tracker(ns.next_id);
            loops.new_scope(end, cond);

            let mut body_reachable = true;

            for stmt in body_stmt {
                statement(
                    stmt,
                    func,
                    cfg,
                    contract_no,
                    ns,
                    vartab,
                    loops,
                    placeholder,
                    return_override,
                    opt,
                );

                body_reachable = stmt.reachable();
            }

            if body_reachable {
                cfg.add(vartab, Instr::Branch { block: cond });
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
                statement(
                    stmt,
                    func,
                    cfg,
                    contract_no,
                    ns,
                    vartab,
                    loops,
                    placeholder,
                    return_override,
                    opt,
                );
            }

            cfg.add(vartab, Instr::Branch { block: body_block });

            cfg.set_basic_block(body_block);

            loops.new_scope(
                end_block,
                if next.is_empty() {
                    body_block
                } else {
                    next_block
                },
            );

            vartab.new_dirty_tracker(ns.next_id);

            let mut body_reachable = true;

            for stmt in body {
                statement(
                    stmt,
                    func,
                    cfg,
                    contract_no,
                    ns,
                    vartab,
                    loops,
                    placeholder,
                    return_override,
                    opt,
                );

                body_reachable = stmt.reachable();
            }

            if body_reachable {
                cfg.add(vartab, Instr::Branch { block: next_block });
            }

            loops.leave_scope();

            if body_reachable {
                cfg.set_basic_block(next_block);

                if !next.is_empty() {
                    for stmt in next {
                        statement(
                            stmt,
                            func,
                            cfg,
                            contract_no,
                            ns,
                            vartab,
                            loops,
                            placeholder,
                            return_override,
                            opt,
                        );
                        body_reachable = stmt.reachable();
                    }
                }

                if body_reachable {
                    cfg.add(vartab, Instr::Branch { block: body_block });
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
                statement(
                    stmt,
                    func,
                    cfg,
                    contract_no,
                    ns,
                    vartab,
                    loops,
                    placeholder,
                    return_override,
                    opt,
                );
            }

            cfg.add(vartab, Instr::Branch { block: cond_block });

            cfg.set_basic_block(cond_block);

            let cond_expr = expression(cond_expr, cfg, contract_no, Some(func), ns, vartab, opt);

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: cond_expr,
                    true_block: body_block,
                    false_block: end_block,
                },
            );

            cfg.set_basic_block(body_block);

            // continue goes to next
            loops.new_scope(end_block, next_block);

            vartab.new_dirty_tracker(ns.next_id);

            let mut body_reachable = true;

            for stmt in body {
                statement(
                    stmt,
                    func,
                    cfg,
                    contract_no,
                    ns,
                    vartab,
                    loops,
                    placeholder,
                    return_override,
                    opt,
                );

                body_reachable = stmt.reachable();
            }

            if body_reachable {
                cfg.add(vartab, Instr::Branch { block: next_block });
            }

            loops.leave_scope();

            cfg.set_basic_block(next_block);

            let mut next_reachable = true;

            for stmt in next {
                statement(
                    stmt,
                    func,
                    cfg,
                    contract_no,
                    ns,
                    vartab,
                    loops,
                    placeholder,
                    return_override,
                    opt,
                );

                next_reachable = stmt.reachable();
            }

            if next_reachable {
                cfg.add(vartab, Instr::Branch { block: cond_block });
            }

            cfg.set_basic_block(end_block);

            let set = vartab.pop_dirty_tracker();
            cfg.set_phis(next_block, set.clone());
            cfg.set_phis(end_block, set.clone());
            cfg.set_phis(cond_block, set);
        }
        Statement::Destructure(_, fields, expr) => {
            destructure(fields, expr, cfg, contract_no, func, ns, vartab, loops, opt)
        }
        Statement::TryCatch(_, _, try_stmt) => try_catch(
            try_stmt,
            func,
            cfg,
            contract_no,
            ns,
            vartab,
            loops,
            placeholder,
            return_override,
            opt,
        ),
        Statement::Emit { event_no, args, .. } => {
            let event = &ns.events[*event_no];
            let mut data = Vec::new();
            let mut data_tys = Vec::new();
            let mut topics = Vec::new();
            let mut topic_tys = Vec::new();

            if !event.anonymous && !ns.target.is_substrate() {
                let mut hasher = Keccak::v256();
                hasher.update(event.signature.as_bytes());
                let mut hash = [0u8; 32];
                hasher.finalize(&mut hash);

                topic_tys.push(Type::Bytes(32));
                topics.push(Expression::BytesLiteral(
                    pt::Loc(0, 0, 0),
                    Type::Bytes(32),
                    hash.to_vec(),
                ));
            }

            for (i, arg) in args.iter().enumerate() {
                if event.fields[i].indexed {
                    let ty = arg.ty();

                    match ty {
                        Type::String | Type::DynamicBytes => {
                            let e = expression(
                                &Expression::Builtin(
                                    pt::Loc(0, 0, 0),
                                    vec![Type::Bytes(32)],
                                    Builtin::Keccak256,
                                    vec![arg.clone()],
                                ),
                                cfg,
                                contract_no,
                                Some(func),
                                ns,
                                vartab,
                                opt,
                            );

                            topics.push(e);
                            topic_tys.push(Type::Bytes(32));
                        }
                        Type::Struct(_) | Type::Array(..) => {
                            // We should have an AbiEncodePackedPad
                            let e = expression(
                                &Expression::Builtin(
                                    pt::Loc(0, 0, 0),
                                    vec![Type::Bytes(32)],
                                    Builtin::Keccak256,
                                    vec![Expression::Builtin(
                                        pt::Loc(0, 0, 0),
                                        vec![Type::DynamicBytes],
                                        Builtin::AbiEncodePacked,
                                        vec![arg.clone()],
                                    )],
                                ),
                                cfg,
                                contract_no,
                                Some(func),
                                ns,
                                vartab,
                                opt,
                            );

                            topics.push(e);
                            topic_tys.push(Type::Bytes(32));
                        }
                        _ => {
                            let e = expression(arg, cfg, contract_no, Some(func), ns, vartab, opt);

                            topics.push(e);
                            topic_tys.push(ty);
                        }
                    }
                } else {
                    let e = expression(arg, cfg, contract_no, Some(func), ns, vartab, opt);

                    data.push(e);
                    data_tys.push(arg.ty());
                }
            }

            cfg.add(
                vartab,
                Instr::EmitEvent {
                    event_no: *event_no,
                    data,
                    data_tys,
                    topics,
                    topic_tys,
                },
            );
        }
        Statement::Underscore(_) => {
            // ensure we get phi nodes for the return values
            if let Some(instr @ Instr::Call { res, .. }) = placeholder {
                for var_no in res {
                    vartab.set_dirty(*var_no);
                }

                cfg.add(vartab, instr.clone());
            } else {
                panic!("placeholder should be provided for modifiers");
            }
        }
    }
}

/// Generate if-then-no-else
fn if_then(
    cond: &Expression,
    then_stmt: &[Statement],
    func: &Function,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    loops: &mut LoopScopes,
    placeholder: Option<&Instr>,
    return_override: Option<&Instr>,
    opt: &Options,
) {
    let cond = expression(cond, cfg, contract_no, Some(func), ns, vartab, opt);

    let then = cfg.new_basic_block("then".to_string());
    let endif = cfg.new_basic_block("endif".to_string());

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_block: then,
            false_block: endif,
        },
    );

    cfg.set_basic_block(then);

    vartab.new_dirty_tracker(ns.next_id);

    let mut reachable = true;

    for stmt in then_stmt {
        statement(
            stmt,
            func,
            cfg,
            contract_no,
            ns,
            vartab,
            loops,
            placeholder,
            return_override,
            opt,
        );

        reachable = stmt.reachable();
    }

    if reachable {
        cfg.add(vartab, Instr::Branch { block: endif });
    }

    cfg.set_phis(endif, vartab.pop_dirty_tracker());

    cfg.set_basic_block(endif);
}

/// Generate if-then-else
fn if_then_else(
    cond: &Expression,
    then_stmt: &[Statement],
    else_stmt: &[Statement],
    func: &Function,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    loops: &mut LoopScopes,
    placeholder: Option<&Instr>,
    return_override: Option<&Instr>,
    opt: &Options,
) {
    let cond = expression(cond, cfg, contract_no, Some(func), ns, vartab, opt);

    let then = cfg.new_basic_block("then".to_string());
    let else_ = cfg.new_basic_block("else".to_string());
    let endif = cfg.new_basic_block("endif".to_string());

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_block: then,
            false_block: else_,
        },
    );

    // then
    cfg.set_basic_block(then);

    vartab.new_dirty_tracker(ns.next_id);

    let mut then_reachable = true;

    for stmt in then_stmt {
        statement(
            stmt,
            func,
            cfg,
            contract_no,
            ns,
            vartab,
            loops,
            placeholder,
            return_override,
            opt,
        );

        then_reachable = stmt.reachable();
    }

    if then_reachable {
        cfg.add(vartab, Instr::Branch { block: endif });
    }

    // else
    cfg.set_basic_block(else_);

    let mut else_reachable = true;

    for stmt in else_stmt {
        statement(
            stmt,
            func,
            cfg,
            contract_no,
            ns,
            vartab,
            loops,
            placeholder,
            return_override,
            opt,
        );

        else_reachable = stmt.reachable();
    }

    if else_reachable {
        cfg.add(vartab, Instr::Branch { block: endif });
    }

    cfg.set_phis(endif, vartab.pop_dirty_tracker());

    cfg.set_basic_block(endif);
}

fn returns(
    expr: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: &Function,
    ns: &Namespace,
    vartab: &mut Vartable,
    loops: &mut LoopScopes,
    opt: &Options,
) {
    // Can only be another function call without returns
    let uncast_values = match expr {
        // Explicitly recurse for ternary expressions.
        // `return a ? b : c` is transformed into pseudo code `a ? return b : return c`
        Expression::Ternary(_, _, cond, left, right) => {
            let cond = expression(cond, cfg, contract_no, Some(func), ns, vartab, opt);

            let left_block = cfg.new_basic_block("left".to_string());
            let right_block = cfg.new_basic_block("right".to_string());

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond,
                    true_block: left_block,
                    false_block: right_block,
                },
            );

            vartab.new_dirty_tracker(ns.next_id);

            cfg.set_basic_block(left_block);
            returns(left, cfg, contract_no, func, ns, vartab, loops, opt);

            cfg.set_basic_block(right_block);
            returns(right, cfg, contract_no, func, ns, vartab, loops, opt);

            return;
        }

        Expression::Builtin(_, _, Builtin::AbiDecode, _)
        | Expression::InternalFunctionCall { .. }
        | Expression::ExternalFunctionCall { .. }
        | Expression::ExternalFunctionCallRaw { .. } => {
            emit_function_call(expr, contract_no, cfg, Some(func), ns, vartab, opt)
        }

        Expression::List(_, exprs) => exprs.clone(),

        // Can be any other expression
        _ => {
            vec![expr.clone()]
        }
    };

    let cast_values = func
        .returns
        .iter()
        .zip(uncast_values.into_iter())
        .map(|(left, right)| {
            let right = cast(&left.loc, right, &left.ty, true, ns, &mut Vec::new())
                .expect("sema should have checked cast");
            // casts to StorageLoad generate LoadStorage instructions
            expression(&right, cfg, contract_no, Some(func), ns, vartab, opt)
        })
        .collect();

    cfg.add(vartab, Instr::Return { value: cast_values });
}

fn destructure(
    fields: &[DestructureField],
    expr: &Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: &Function,
    ns: &Namespace,
    vartab: &mut Vartable,
    loops: &mut LoopScopes,
    opt: &Options,
) {
    if let Expression::Ternary(_, _, cond, left, right) = expr {
        let cond = expression(cond, cfg, contract_no, Some(func), ns, vartab, opt);

        let left_block = cfg.new_basic_block("left".to_string());
        let right_block = cfg.new_basic_block("right".to_string());
        let done_block = cfg.new_basic_block("done".to_string());

        cfg.add(
            vartab,
            Instr::BranchCond {
                cond,
                true_block: left_block,
                false_block: right_block,
            },
        );

        vartab.new_dirty_tracker(ns.next_id);

        cfg.set_basic_block(left_block);

        destructure(fields, left, cfg, contract_no, func, ns, vartab, loops, opt);

        cfg.add(vartab, Instr::Branch { block: done_block });

        cfg.set_basic_block(right_block);

        destructure(
            fields,
            right,
            cfg,
            contract_no,
            func,
            ns,
            vartab,
            loops,
            opt,
        );

        cfg.add(vartab, Instr::Branch { block: done_block });

        cfg.set_phis(done_block, vartab.pop_dirty_tracker());

        cfg.set_basic_block(done_block);

        return;
    }

    let mut values = match expr {
        Expression::List(_, exprs) => {
            let mut values = Vec::new();

            for expr in exprs {
                let loc = expr.loc();
                let expr = expression(expr, cfg, contract_no, Some(func), ns, vartab, opt);
                let ty = expr.ty();

                let res = vartab.temp_anonymous(&ty);

                cfg.add(vartab, Instr::Set { loc, res, expr });

                values.push(Expression::Variable(loc, ty, res));
            }

            values
        }
        _ => {
            // must be function call, either internal or external
            emit_function_call(expr, contract_no, cfg, Some(func), ns, vartab, opt)
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
                let expr = cast(&param.loc, right, &param.ty, true, ns, &mut Vec::new())
                    .expect("sema should have checked cast");
                // casts to StorageLoad generate LoadStorage instructions
                let expr = expression(&expr, cfg, contract_no, Some(func), ns, vartab, opt);

                if should_remove_variable(res, func, opt) {
                    continue;
                }

                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: param.name_loc.unwrap_or(pt::Loc(0, 0, 0)),
                        res: *res,
                        expr,
                    },
                );
            }
            DestructureField::Expression(left) => {
                // the resolver did not cast the expression
                let loc = left.loc();

                let expr = cast(
                    &loc,
                    right,
                    left.ty().deref_any(),
                    true,
                    ns,
                    &mut Vec::new(),
                )
                .expect("sema should have checked cast");
                // casts to StorageLoad generate LoadStorage instructions
                let expr = expression(&expr, cfg, contract_no, Some(func), ns, vartab, opt);

                if should_remove_assignment(ns, left, func, opt) {
                    continue;
                }

                assign_single(left, &expr, cfg, contract_no, Some(func), ns, vartab, opt);
            }
        }
    }
}

/// Resolve try catch statement
fn try_catch(
    try_stmt: &TryCatch,
    func: &Function,
    cfg: &mut ControlFlowGraph,
    callee_contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    loops: &mut LoopScopes,
    placeholder: Option<&Instr>,
    return_override: Option<&Instr>,
    opt: &Options,
) {
    let success = vartab.temp(
        &pt::Identifier {
            loc: try_stmt.expr.loc(),
            name: "success".to_owned(),
        },
        &Type::Bool,
    );

    let success_block = cfg.new_basic_block("success".to_string());
    let catch_block = cfg.new_basic_block("catch".to_string());
    let finally_block = cfg.new_basic_block("finally".to_string());

    match &try_stmt.expr {
        Expression::ExternalFunctionCall {
            loc,
            function,
            args,
            value,
            gas,
            ..
        } => {
            if let Type::ExternalFunction {
                returns: func_returns,
                ..
            } = function.ty()
            {
                let value = if let Some(value) = value {
                    expression(value, cfg, callee_contract_no, Some(func), ns, vartab, opt)
                } else {
                    Expression::NumberLiteral(pt::Loc(0, 0, 0), Type::Value, BigInt::zero())
                };
                let gas = if let Some(gas) = gas {
                    expression(gas, cfg, callee_contract_no, Some(func), ns, vartab, opt)
                } else {
                    default_gas(ns)
                };
                let function = expression(
                    function,
                    cfg,
                    callee_contract_no,
                    Some(func),
                    ns,
                    vartab,
                    opt,
                );

                let mut tys: Vec<Type> = args.iter().map(|a| a.ty()).collect();

                tys.insert(0, Type::Bytes(4));

                let args = args
                    .iter()
                    .map(|a| expression(a, cfg, callee_contract_no, Some(func), ns, vartab, opt))
                    .collect();

                let selector = Expression::Builtin(
                    *loc,
                    vec![Type::Bytes(4)],
                    Builtin::FunctionSelector,
                    vec![function.clone()],
                );

                let address = Expression::Builtin(
                    *loc,
                    vec![Type::Address(false)],
                    Builtin::ExternalFunctionAddress,
                    vec![function],
                );

                let payload = Expression::AbiEncode {
                    loc: *loc,
                    tys,
                    packed: vec![selector],
                    args,
                };

                cfg.add(
                    vartab,
                    Instr::ExternalCall {
                        success: Some(success),
                        address: Some(address),
                        payload,
                        value,
                        gas,
                        callty: CallTy::Regular,
                    },
                );

                cfg.add(
                    vartab,
                    Instr::BranchCond {
                        cond: Expression::Variable(try_stmt.expr.loc(), Type::Bool, success),
                        true_block: success_block,
                        false_block: catch_block,
                    },
                );

                cfg.set_basic_block(success_block);

                if func_returns != vec![Type::Void] {
                    let mut res = Vec::new();

                    for ret in &try_stmt.returns {
                        res.push(match ret {
                            (Some(pos), _) => *pos,
                            (None, param) => vartab.temp_anonymous(&param.ty),
                        });
                    }

                    let tys = func_returns
                        .iter()
                        .map(|ty| Parameter {
                            ty: ty.clone(),
                            name: String::new(),
                            ty_loc: pt::Loc(0, 0, 0),
                            name_loc: None,
                            loc: pt::Loc(0, 0, 0),
                            indexed: false,
                            readonly: false,
                        })
                        .collect();

                    cfg.add(
                        vartab,
                        Instr::AbiDecode {
                            res,
                            selector: None,
                            exception_block: None,
                            tys,
                            data: Expression::ReturnData(pt::Loc(0, 0, 0)),
                        },
                    );
                }
            } else {
                // dynamic dispatch
                unimplemented!();
            }
        }
        Expression::Constructor {
            contract_no,
            constructor_no,
            args,
            value,
            gas,
            salt,
            space,
            ..
        } => {
            let address_res = match try_stmt.returns.get(0) {
                Some((Some(pos), _)) => *pos,
                _ => vartab.temp_anonymous(&Type::Contract(*contract_no)),
            };

            let value = value.as_ref().map(|value| {
                expression(value, cfg, callee_contract_no, Some(func), ns, vartab, opt)
            });

            let gas = if let Some(gas) = gas {
                expression(gas, cfg, callee_contract_no, Some(func), ns, vartab, opt)
            } else {
                default_gas(ns)
            };
            let salt = salt
                .as_ref()
                .map(|salt| expression(salt, cfg, callee_contract_no, Some(func), ns, vartab, opt));
            let space = space.as_ref().map(|space| {
                expression(space, cfg, callee_contract_no, Some(func), ns, vartab, opt)
            });

            let args = args
                .iter()
                .map(|a| expression(a, cfg, callee_contract_no, Some(func), ns, vartab, opt))
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
                    space,
                },
            );

            cfg.add(
                vartab,
                Instr::BranchCond {
                    cond: Expression::Variable(try_stmt.expr.loc(), Type::Bool, success),
                    true_block: success_block,
                    false_block: catch_block,
                },
            );

            cfg.set_basic_block(success_block);
        }
        _ => unreachable!(),
    }

    vartab.new_dirty_tracker(ns.next_id);

    let mut finally_reachable = true;

    for stmt in &try_stmt.ok_stmt {
        statement(
            stmt,
            func,
            cfg,
            callee_contract_no,
            ns,
            vartab,
            loops,
            placeholder,
            return_override,
            opt,
        );

        finally_reachable = stmt.reachable();
    }

    if finally_reachable {
        cfg.add(
            vartab,
            Instr::Branch {
                block: finally_block,
            },
        );
    }

    cfg.set_basic_block(catch_block);

    if let Some((error_param_pos, error_param, error_stmt)) = &try_stmt.error {
        let no_reason_block = cfg.new_basic_block("no_reason".to_string());

        let error_var = match error_param_pos {
            Some(pos) => *pos,
            _ => vartab.temp_anonymous(&Type::String),
        };

        cfg.add(
            vartab,
            Instr::AbiDecode {
                selector: Some(0x08c3_79a0),
                exception_block: Some(no_reason_block),
                res: vec![error_var],
                tys: vec![error_param.clone()],
                data: Expression::ReturnData(pt::Loc(0, 0, 0)),
            },
        );

        let mut reachable = true;

        for stmt in error_stmt {
            statement(
                stmt,
                func,
                cfg,
                callee_contract_no,
                ns,
                vartab,
                loops,
                placeholder,
                return_override,
                opt,
            );

            reachable = stmt.reachable();
        }
        if reachable {
            cfg.add(
                vartab,
                Instr::Branch {
                    block: finally_block,
                },
            );
        }

        cfg.set_basic_block(no_reason_block);
    }

    if let Some(res) = try_stmt.catch_param_pos {
        cfg.add(
            vartab,
            Instr::Set {
                loc: pt::Loc(0, 0, 0),
                res,
                expr: Expression::ReturnData(pt::Loc(0, 0, 0)),
            },
        );
    }

    let mut reachable = true;

    for stmt in &try_stmt.catch_stmt {
        statement(
            stmt,
            func,
            cfg,
            callee_contract_no,
            ns,
            vartab,
            loops,
            placeholder,
            return_override,
            opt,
        );

        reachable = stmt.reachable();
    }

    if reachable {
        cfg.add(
            vartab,
            Instr::Branch {
                block: finally_block,
            },
        );
    }

    let mut set = vartab.pop_dirty_tracker();
    if let Some(pos) = &try_stmt.catch_param_pos {
        set.remove(pos);
    }
    if let Some((Some(pos), _, _)) = &try_stmt.error {
        set.remove(pos);
    }
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
    /// Default value for a type, e.g. an empty string. Some types cannot have a default value,
    /// for example a reference to a variable in storage.
    pub fn default(&self, ns: &Namespace) -> Option<Expression> {
        match self {
            Type::Address(_) | Type::Uint(_) | Type::Int(_) => Some(Expression::NumberLiteral(
                pt::Loc(0, 0, 0),
                self.clone(),
                BigInt::from(0),
            )),
            Type::Bool => Some(Expression::BoolLiteral(pt::Loc(0, 0, 0), false)),
            Type::Bytes(n) => {
                let mut l = Vec::new();
                l.resize(*n as usize, 0);
                Some(Expression::BytesLiteral(pt::Loc(0, 0, 0), self.clone(), l))
            }
            Type::Enum(e) => ns.enums[*e].ty.default(ns),
            Type::Struct(struct_no) => {
                // make sure all our fields have default values
                for field in &ns.structs[*struct_no].fields {
                    field.ty.default(ns)?;
                }

                Some(Expression::StructLiteral(
                    pt::Loc(0, 0, 0),
                    self.clone(),
                    Vec::new(),
                ))
            }
            Type::Ref(_) => unreachable!(),
            Type::StorageRef(_, _) => None,
            Type::String | Type::DynamicBytes => Some(Expression::AllocDynamicArray(
                pt::Loc(0, 0, 0),
                self.clone(),
                Box::new(Expression::NumberLiteral(
                    pt::Loc(0, 0, 0),
                    Type::Uint(32),
                    BigInt::zero(),
                )),
                None,
            )),
            Type::InternalFunction { .. } | Type::Contract(_) | Type::ExternalFunction { .. } => {
                None
            }
            Type::Array(ty, dims) => {
                ty.default(ns)?;

                if dims[0].is_none() {
                    Some(Expression::AllocDynamicArray(
                        pt::Loc(0, 0, 0),
                        self.clone(),
                        Box::new(Expression::NumberLiteral(
                            pt::Loc(0, 0, 0),
                            Type::Uint(32),
                            BigInt::zero(),
                        )),
                        None,
                    ))
                } else {
                    Some(Expression::ArrayLiteral(
                        pt::Loc(0, 0, 0),
                        self.clone(),
                        Vec::new(),
                        Vec::new(),
                    ))
                }
            }
            _ => None,
        }
    }
}

impl Namespace {
    /// Phoney default constructor
    pub fn default_constructor(&self, contract_no: usize) -> Function {
        let mut func = Function::new(
            pt::Loc(0, 0, 0),
            "".to_owned(),
            Some(contract_no),
            vec![],
            pt::FunctionTy::Constructor,
            None,
            pt::Visibility::Public(None),
            Vec::new(),
            Vec::new(),
            self,
        );

        func.body = vec![Statement::Return(pt::Loc(0, 0, 0), None)];
        func.has_body = true;

        func
    }
}

/// This function looks for expressions that have side effects during code execution and
/// processes them.
/// They must be added to the cfg event if we remove the assignment
pub fn process_side_effects_expressions(
    exp: &Expression,
    ctx: &mut SideEffectsCheckParameters,
) -> bool {
    match &exp {
        Expression::InternalFunctionCall { .. }
        | Expression::ExternalFunctionCall { .. }
        | Expression::ExternalFunctionCallRaw { .. }
        | Expression::Constructor { .. }
        | Expression::Assign(..) => {
            let _ = expression(
                exp,
                ctx.cfg,
                ctx.contract_no,
                ctx.func,
                ctx.ns,
                ctx.vartab,
                ctx.opt,
            );
            false
        }

        Expression::Builtin(_, _, builtin_type, _) => match &builtin_type {
            Builtin::PayableSend
            | Builtin::ArrayPush
            | Builtin::ArrayPop
            // PayableTransfer, Revert, Require and SelfDestruct do not occur inside an expression
            // for they return no value. They should not bother the unused variable elimination.
            | Builtin::PayableTransfer
            | Builtin::Revert
            | Builtin::Require
            | Builtin::SelfDestruct
            | Builtin::WriteInt8
            | Builtin::WriteInt16LE
            | Builtin::WriteInt32LE
            | Builtin::WriteInt64LE
            | Builtin::WriteInt128LE
            | Builtin::WriteInt256LE
            | Builtin::WriteUint16LE
            | Builtin::WriteUint32LE
            | Builtin::WriteUint64LE
            | Builtin::WriteUint128LE
            | Builtin::WriteUint256LE
            | Builtin::WriteAddress => {
                let _ = expression(exp, ctx.cfg, ctx.contract_no, ctx.func, ctx.ns, ctx.vartab, ctx.opt);
                false
            }

            _ => true,
        },

        _ => true,
    }
}
