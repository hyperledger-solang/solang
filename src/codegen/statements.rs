// SPDX-License-Identifier: Apache-2.0

use num_bigint::BigInt;

use super::encoding::abi_encode;
use super::expression::{assign_single, default_gas, emit_function_call, expression};
use super::Options;
use super::{
    cfg::{ControlFlowGraph, Instr},
    vartable::Vartable,
};
use crate::codegen::constructor::call_constructor;
use crate::codegen::events::new_event_emitter;
use crate::codegen::unused_variable::{
    should_remove_assignment, should_remove_variable, SideEffectsCheckParameters,
};
use crate::codegen::yul::inline_assembly_cfg;
use crate::codegen::Expression;
use crate::sema::ast;
use crate::sema::ast::RetrieveType;
use crate::sema::ast::{
    ArrayLength, CallTy, DestructureField, Function, Namespace, Parameter, Statement, TryCatch,
    Type,
};
use crate::sema::Recurse;
use num_traits::Zero;
use solang_parser::pt;
use solang_parser::pt::CodeLocation;

/// Resolve a statement, which might be a block of statements or an entire body of a function
pub(crate) fn statement(
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

            let mut expression = expression(init, cfg, contract_no, Some(func), ns, vartab, opt);

            // Let's check if the declaration is a declaration of a dynamic array
            if let Expression::AllocDynamicBytes(
                loc_dyn_arr,
                ty_dyn_arr @ Type::Array(..),
                size,
                opt,
            ) = expression
            {
                let temp_res = vartab.temp_name("array_length", &Type::Uint(32));

                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: *loc,
                        res: temp_res,
                        expr: *size,
                    },
                );
                // If expression is an AllocDynamic array, replace the expression with AllocDynamicArray(_,_,tempvar,_) to avoid inserting size twice in the cfg
                expression = Expression::AllocDynamicBytes(
                    loc_dyn_arr,
                    ty_dyn_arr,
                    Box::new(Expression::Variable(*loc, Type::Uint(32), temp_res)),
                    opt,
                );
                cfg.array_lengths_temps.insert(*pos, temp_res);
            } else if let Expression::Variable(_, _, res) = &expression {
                // If declaration happens with an existing array, check if the size of the array is known.
                // If the size of the right hand side is known (is in the array_length_map), make the left hand side track it
                // Now, we will have two keys in the map that point to the same temporary variable
                if let Some(to_add) = cfg.array_lengths_temps.clone().get(res) {
                    cfg.array_lengths_temps.insert(*pos, *to_add);
                }
            }

            cfg.add(
                vartab,
                Instr::Set {
                    loc: *loc,
                    res: *pos,
                    expr: expression,
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
            // Handling arrays without size, defaulting the initial size with zero

            if matches!(param.ty, Type::Array(..)) {
                let num =
                    Expression::NumberLiteral(pt::Loc::Codegen, Type::Uint(32), BigInt::zero());
                let temp_res = vartab.temp_name("array_length", &Type::Uint(32));
                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: *loc,
                        res: temp_res,
                        expr: num,
                    },
                );
                cfg.array_lengths_temps.insert(*pos, temp_res);
            }
        }
        Statement::Return(_, expr) => {
            if let Some(return_instr) = return_override {
                cfg.add(vartab, return_instr.clone());
            } else {
                match expr {
                    None => cfg.add(vartab, Instr::Return { value: Vec::new() }),
                    Some(expr) => returns(expr, cfg, contract_no, func, ns, vartab, opt),
                }
            }
        }
        Statement::Expression(_, reachable, expr) => {
            if let ast::Expression::Assign { left, right, .. } = &expr {
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

            vartab.new_dirty_tracker();
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

            vartab.new_dirty_tracker();
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

            vartab.new_dirty_tracker();

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

            vartab.new_dirty_tracker();

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
            destructure(fields, expr, cfg, contract_no, func, ns, vartab, opt)
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
        Statement::Emit {
            loc,
            event_no,
            args,
            ..
        } => {
            let emitter = new_event_emitter(loc, *event_no, args, ns);
            emitter.emit(contract_no, func, cfg, vartab, opt);
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

        Statement::Assembly(inline_assembly, ..) => {
            inline_assembly_cfg(inline_assembly, contract_no, ns, cfg, vartab, opt);
        }
    }
}

/// Generate if-then-no-else
fn if_then(
    cond: &ast::Expression,
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

    vartab.new_dirty_tracker();

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
    cond: &ast::Expression,
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

    vartab.new_dirty_tracker();

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
    expr: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: &Function,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) {
    // Can only be another function call without returns
    let uncast_values = match expr {
        // Explicitly recurse for conditinal operator expressions.
        // `return a ? b : c` is transformed into pseudo code `a ? return b : return c`
        ast::Expression::ConditionalOperator {
            cond,
            true_option: left,
            false_option: right,
            ..
        } => {
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

            vartab.new_dirty_tracker();

            cfg.set_basic_block(left_block);
            returns(left, cfg, contract_no, func, ns, vartab, opt);

            cfg.set_basic_block(right_block);
            returns(right, cfg, contract_no, func, ns, vartab, opt);

            return;
        }

        ast::Expression::Builtin {
            kind: ast::Builtin::AbiDecode,
            ..
        }
        | ast::Expression::InternalFunctionCall { .. }
        | ast::Expression::ExternalFunctionCall { .. }
        | ast::Expression::ExternalFunctionCallRaw { .. } => {
            emit_function_call(expr, contract_no, cfg, Some(func), ns, vartab, opt)
        }

        ast::Expression::List { list, .. } => list
            .iter()
            .map(|e| expression(e, cfg, contract_no, Some(func), ns, vartab, opt))
            .collect::<Vec<Expression>>(),

        // Can be any other expression
        _ => {
            vec![expression(
                expr,
                cfg,
                contract_no,
                Some(func),
                ns,
                vartab,
                opt,
            )]
        }
    };

    let cast_values = func
        .returns
        .iter()
        .zip(uncast_values.into_iter())
        .map(|(left, right)| try_load_and_cast(&right.loc(), &right, &left.ty, ns, cfg, vartab))
        .collect();

    cfg.add(vartab, Instr::Return { value: cast_values });
}

fn destructure(
    fields: &[DestructureField],
    expr: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: &Function,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) {
    if let ast::Expression::ConditionalOperator {
        cond,
        true_option: left,
        false_option: right,
        ..
    } = expr
    {
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

        vartab.new_dirty_tracker();

        cfg.set_basic_block(left_block);

        destructure(fields, left, cfg, contract_no, func, ns, vartab, opt);

        cfg.add(vartab, Instr::Branch { block: done_block });

        cfg.set_basic_block(right_block);

        destructure(fields, right, cfg, contract_no, func, ns, vartab, opt);

        cfg.add(vartab, Instr::Branch { block: done_block });

        cfg.set_phis(done_block, vartab.pop_dirty_tracker());

        cfg.set_basic_block(done_block);

        return;
    }

    let mut values = match expr {
        ast::Expression::List { list, .. } => {
            let mut values = Vec::new();

            for expr in list {
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
                let expr = try_load_and_cast(&param.loc, &right, &param.ty, ns, cfg, vartab);

                if should_remove_variable(res, func, opt) {
                    continue;
                }

                cfg.add(
                    vartab,
                    Instr::Set {
                        loc: param.loc,
                        res: *res,
                        expr,
                    },
                );
            }
            DestructureField::Expression(left) => {
                let expr = try_load_and_cast(&left.loc(), &right, &left.ty(), ns, cfg, vartab);

                if should_remove_assignment(ns, left, func, opt) {
                    continue;
                }

                assign_single(left, expr, cfg, contract_no, Some(func), ns, vartab, opt);
            }
        }
    }
}

/// During a destructure statement, sema only checks if the cast is possible. During codegen, we
/// perform the real cast and add an instruction to the CFG to load a value from the storage if want it.
/// The existing codegen cast function does not manage the CFG, so the loads must be done here.
fn try_load_and_cast(
    loc: &pt::Loc,
    expr: &Expression,
    to_ty: &Type,
    ns: &Namespace,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) -> Expression {
    match expr.ty() {
        Type::StorageRef(_, ty) => {
            if let Expression::Subscript(_, _, ty, ..) = &expr {
                if ty.is_storage_bytes() {
                    return expr.cast(to_ty, ns);
                }
            }

            if matches!(to_ty, Type::StorageRef(..)) {
                // If we want a storage reference, there is no need to load from storage
                return expr.cast(to_ty, ns);
            }

            let anonymous_no = vartab.temp_anonymous(&ty);
            cfg.add(
                vartab,
                Instr::LoadStorage {
                    res: anonymous_no,
                    ty: (*ty).clone(),
                    storage: expr.cast(to_ty, ns),
                },
            );

            Expression::Variable(*loc, (*ty).clone(), anonymous_no)
        }
        Type::Ref(ty) => match *ty {
            Type::Array(_, _) => expr.cast(to_ty, ns),
            _ => Expression::Load(pt::Loc::Builtin, *ty, expr.clone().into()).cast(to_ty, ns),
        },
        _ => expr.cast(to_ty, ns),
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
        ast::Expression::ExternalFunctionCall {
            loc,
            function,
            args,
            call_args,
            ..
        } => {
            if let Type::ExternalFunction {
                returns: func_returns,
                ..
            } = function.ty()
            {
                let value = if let Some(value) = &call_args.value {
                    expression(value, cfg, callee_contract_no, Some(func), ns, vartab, opt)
                } else {
                    Expression::NumberLiteral(pt::Loc::Codegen, Type::Value, BigInt::zero())
                };
                let gas = if let Some(gas) = &call_args.gas {
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

                let mut args = args
                    .iter()
                    .map(|a| expression(a, cfg, callee_contract_no, Some(func), ns, vartab, opt))
                    .collect::<Vec<Expression>>();

                let selector = function.external_function_selector();

                let address = function.external_function_address();

                args.insert(0, selector);
                let (payload, _) = abi_encode(loc, args, ns, vartab, cfg, false);

                cfg.add(
                    vartab,
                    Instr::ExternalCall {
                        success: Some(success),
                        address: Some(address),
                        accounts: None,
                        seeds: None,
                        payload,
                        value,
                        gas,
                        callty: CallTy::Regular,
                        contract_function_no: None,
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
                            id: None,
                            ty_loc: Some(pt::Loc::Codegen),
                            loc: pt::Loc::Codegen,
                            indexed: false,
                            readonly: false,
                            recursive: false,
                        })
                        .collect();

                    cfg.add(
                        vartab,
                        Instr::AbiDecode {
                            res,
                            selector: None,
                            exception_block: None,
                            tys,
                            data: Expression::ReturnData(pt::Loc::Codegen),
                            data_len: None,
                        },
                    );
                }
            } else {
                // dynamic dispatch
                unimplemented!();
            }
        }
        ast::Expression::Constructor {
            loc,
            contract_no,
            constructor_no,
            args,
            call_args,
            ..
        } => {
            let address_res = match try_stmt.returns.get(0) {
                Some((Some(pos), _)) => *pos,
                _ => vartab.temp_anonymous(&Type::Contract(*contract_no)),
            };

            call_constructor(
                loc,
                contract_no,
                callee_contract_no,
                constructor_no,
                args,
                call_args,
                address_res,
                Some(success),
                Some(func),
                ns,
                vartab,
                cfg,
                opt,
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

    vartab.new_dirty_tracker();

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

    for (error_param_pos, error_param, error_stmt) in &try_stmt.errors {
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
                data: Expression::ReturnData(pt::Loc::Codegen),
                data_len: None,
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
                loc: pt::Loc::Codegen,
                res,
                expr: Expression::ReturnData(pt::Loc::Codegen),
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
    for (pos, _, _) in &try_stmt.errors {
        if let Some(pos) = pos {
            set.remove(pos);
        }
    }
    cfg.set_phis(finally_block, set);

    cfg.set_basic_block(finally_block);
}

pub struct LoopScope {
    break_bb: usize,
    continue_bb: usize,
}

pub struct LoopScopes(Vec<LoopScope>);

impl LoopScopes {
    pub fn new() -> Self {
        LoopScopes(Vec::new())
    }

    pub(crate) fn new_scope(&mut self, break_bb: usize, continue_bb: usize) {
        self.0.push(LoopScope {
            break_bb,
            continue_bb,
        })
    }

    pub(crate) fn leave_scope(&mut self) -> LoopScope {
        self.0.pop().expect("should be in loop scope")
    }

    pub(crate) fn do_break(&mut self) -> usize {
        self.0.last().unwrap().break_bb
    }

    pub(crate) fn do_continue(&mut self) -> usize {
        self.0.last().unwrap().continue_bb
    }
}

impl Type {
    /// Default value for a type, e.g. an empty string. Some types cannot have a default value,
    /// for example a reference to a variable in storage.
    pub fn default(&self, ns: &Namespace) -> Option<Expression> {
        match self {
            Type::Address(_) | Type::Uint(_) | Type::Int(_) => Some(Expression::NumberLiteral(
                pt::Loc::Codegen,
                self.clone(),
                BigInt::from(0),
            )),
            Type::Bool => Some(Expression::BoolLiteral(pt::Loc::Codegen, false)),
            Type::Bytes(n) => {
                let mut l = Vec::new();
                l.resize(*n as usize, 0);
                Some(Expression::BytesLiteral(pt::Loc::Codegen, self.clone(), l))
            }
            Type::Enum(e) => ns.enums[*e].ty.default(ns),
            Type::Struct(struct_ty) => {
                // make sure all our fields have default values
                for field in &struct_ty.definition(ns).fields {
                    field.ty.default(ns)?;
                }

                Some(Expression::StructLiteral(
                    pt::Loc::Codegen,
                    self.clone(),
                    Vec::new(),
                ))
            }
            Type::Ref(ty) => {
                assert!(matches!(ty.as_ref(), Type::Address(_)));

                Some(Expression::GetRef(
                    pt::Loc::Codegen,
                    Type::Ref(Box::new(ty.as_ref().clone())),
                    Box::new(Expression::NumberLiteral(
                        pt::Loc::Codegen,
                        ty.as_ref().clone(),
                        BigInt::from(0),
                    )),
                ))
            }
            Type::StorageRef(..) => None,
            Type::String | Type::DynamicBytes => Some(Expression::AllocDynamicBytes(
                pt::Loc::Codegen,
                self.clone(),
                Box::new(Expression::NumberLiteral(
                    pt::Loc::Codegen,
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

                if dims.last() == Some(&ArrayLength::Dynamic) {
                    Some(Expression::AllocDynamicBytes(
                        pt::Loc::Codegen,
                        self.clone(),
                        Box::new(Expression::NumberLiteral(
                            pt::Loc::Codegen,
                            Type::Uint(32),
                            BigInt::zero(),
                        )),
                        None,
                    ))
                } else {
                    Some(Expression::ArrayLiteral(
                        pt::Loc::Codegen,
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
            pt::Loc::Codegen,
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

        func.body = vec![Statement::Return(pt::Loc::Codegen, None)];
        func.has_body = true;

        func
    }
}

/// This function looks for expressions that have side effects during code execution and
/// processes them.
/// They must be added to the cfg event if we remove the assignment
pub fn process_side_effects_expressions(
    exp: &ast::Expression,
    ctx: &mut SideEffectsCheckParameters,
) -> bool {
    match &exp {
        ast::Expression::InternalFunctionCall { .. }
        | ast::Expression::ExternalFunctionCall { .. }
        | ast::Expression::ExternalFunctionCallRaw { .. }
        | ast::Expression::Constructor { .. }
        | ast::Expression::Assign { .. } => {
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

        ast::Expression::Builtin {
            kind: builtin_type, ..
        } => match &builtin_type {
            ast::Builtin::PayableSend
            | ast::Builtin::ArrayPush
            | ast::Builtin::ArrayPop
            // PayableTransfer, Revert, Require and SelfDestruct do not occur inside an expression
            // for they return no value. They should not bother the unused variable elimination.
            | ast::Builtin::PayableTransfer
            | ast::Builtin::Revert
            | ast::Builtin::Require
            | ast::Builtin::SelfDestruct
            | ast::Builtin::WriteInt8
            | ast::Builtin::WriteInt16LE
            | ast::Builtin::WriteInt32LE
            | ast::Builtin::WriteInt64LE
            | ast::Builtin::WriteInt128LE
            | ast::Builtin::WriteInt256LE
            | ast::Builtin::WriteUint16LE
            | ast::Builtin::WriteUint32LE
            | ast::Builtin::WriteUint64LE
            | ast::Builtin::WriteUint128LE
            | ast::Builtin::WriteUint256LE
            | ast::Builtin::WriteAddress => {
                let _ = expression(exp, ctx.cfg, ctx.contract_no, ctx.func, ctx.ns, ctx.vartab, ctx.opt);
                false
            }

            _ => true,
        },

        _ => true,
    }
}
