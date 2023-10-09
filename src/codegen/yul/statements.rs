// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::statements::LoopScopes;
use crate::codegen::vartable::Vartable;
use crate::codegen::yul::builtin::process_builtin;
use crate::codegen::yul::expression::{expression, process_function_call};
use crate::codegen::{Expression, Options};
use crate::sema::ast::{Namespace, RetrieveType, Type};
use crate::sema::yul::ast;
use crate::sema::yul::ast::{CaseBlock, YulBlock, YulExpression, YulStatement, YulSuffix};
use num_bigint::BigInt;
use num_traits::FromPrimitive;
use solang_parser::pt;
use solang_parser::pt::StorageLocation;

/// Transform YUL statements into CFG instructions
pub(crate) fn statement(
    yul_statement: &YulStatement,
    contract_no: usize,
    loops: &mut LoopScopes,
    ns: &Namespace,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    early_return: &Option<Instr>,
    opt: &Options,
) {
    if !yul_statement.is_reachable() {
        return;
    }

    match yul_statement {
        YulStatement::FunctionCall(_, _, func_no, args) => {
            let returns = process_function_call(*func_no, args, contract_no, vartab, cfg, ns, opt);
            assert_eq!(returns.len(), 1);
            assert_eq!(returns[0], Expression::Poison);
        }

        YulStatement::BuiltInCall(loc, _, builtin_ty, args) => {
            let expr = process_builtin(loc, *builtin_ty, args, contract_no, ns, vartab, cfg, opt);
            assert_eq!(expr, Expression::Poison);
        }

        YulStatement::Block(block) => {
            for item in &block.statements {
                statement(item, contract_no, loops, ns, cfg, vartab, early_return, opt);
            }
        }

        YulStatement::VariableDeclaration(loc, _, vars, init) => {
            process_variable_declaration(loc, vars, init, contract_no, ns, cfg, vartab, opt);
        }

        YulStatement::Assignment(loc, _, lhs, rhs) => {
            process_assignment(loc, lhs, rhs, contract_no, ns, cfg, vartab, opt)
        }

        YulStatement::IfBlock(_, _, condition, block) => process_if_block(
            condition,
            block,
            contract_no,
            loops,
            ns,
            cfg,
            vartab,
            early_return,
            opt,
        ),

        YulStatement::Switch {
            condition,
            cases,
            default,
            ..
        } => switch(
            condition,
            cases,
            default,
            loops,
            contract_no,
            ns,
            vartab,
            cfg,
            early_return,
            opt,
        ),

        YulStatement::For {
            loc,
            init_block,
            post_block,
            condition,
            execution_block,
            ..
        } => process_for_block(
            loc,
            init_block,
            condition,
            post_block,
            execution_block,
            contract_no,
            ns,
            loops,
            cfg,
            vartab,
            early_return,
            opt,
        ),

        YulStatement::Leave(..) => {
            if let Some(early_leave) = early_return {
                cfg.add(vartab, early_leave.clone());
            } else {
                cfg.add(vartab, Instr::Return { value: vec![] });
            }
        }

        YulStatement::Break(..) => {
            cfg.add(
                vartab,
                Instr::Branch {
                    block: loops.do_break(),
                },
            );
        }

        YulStatement::Continue(..) => {
            cfg.add(
                vartab,
                Instr::Branch {
                    block: loops.do_continue(),
                },
            );
        }
    }
}

/// Add variable declaration to the CFG
fn process_variable_declaration(
    loc: &pt::Loc,
    vars: &[(usize, Type)],
    init: &Option<ast::YulExpression>,
    contract_no: usize,
    ns: &Namespace,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    opt: &Options,
) {
    let initializer = if let Some(expr) = init {
        if let ast::YulExpression::FunctionCall(_, func_no, args, _) = expr {
            process_function_call(*func_no, args, contract_no, vartab, cfg, ns, opt)
        } else {
            vec![expression(expr, contract_no, ns, vartab, cfg, opt)]
        }
    } else {
        let mut inits: Vec<Expression> = Vec::with_capacity(vars.len());
        for item in vars {
            inits.push(Expression::Undefined { ty: item.1.clone() });
        }

        inits
    };

    for (var_index, item) in vars.iter().enumerate() {
        cfg.add(
            vartab,
            Instr::Set {
                loc: *loc,
                res: item.0,
                expr: initializer[var_index].clone().cast(&item.1, ns),
            },
        );
    }
}

/// Add assignments to the CFG
fn process_assignment(
    loc: &pt::Loc,
    lhs: &[ast::YulExpression],
    rhs: &ast::YulExpression,
    contract_no: usize,
    ns: &Namespace,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    opt: &Options,
) {
    if lhs.len() > 1 {
        // builtins with multiple returns are not implemented (yet)
        let returns = if let ast::YulExpression::FunctionCall(_, func_no, args, _) = rhs {
            process_function_call(*func_no, args, contract_no, vartab, cfg, ns, opt)
        } else {
            unreachable!("only function call return multiple values");
        };

        for (lhs_no, lhs_item) in lhs.iter().enumerate() {
            cfg_single_assigment(loc, lhs_item, returns[lhs_no].clone(), ns, cfg, vartab);
        }
        return;
    }

    let codegen_rhs = expression(rhs, contract_no, ns, vartab, cfg, opt);
    cfg_single_assigment(loc, &lhs[0], codegen_rhs, ns, cfg, vartab);
}

/// As YUL assignments may contain multiple variables, this function treats one assignment at a time.
fn cfg_single_assigment(
    loc: &pt::Loc,
    lhs: &ast::YulExpression,
    rhs: Expression,
    ns: &Namespace,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) {
    match lhs {
        ast::YulExpression::YulLocalVariable(_, ty, var_no)
        | ast::YulExpression::SolidityLocalVariable(_, ty, None, var_no) => {
            // Ensure both types are compatible
            let rhs = rhs.cast(ty, ns);
            cfg.add(
                vartab,
                Instr::Set {
                    loc: *loc,
                    res: *var_no,
                    expr: rhs,
                },
            );
        }

        ast::YulExpression::SolidityLocalVariable(
            _,
            ty,
            Some(StorageLocation::Memory(_)),
            var_no,
        ) => {
            // This is an assignment to a pointer, so we make sure the rhs has a compatible size
            let rhs = rhs.cast(ty, ns);
            cfg.add(
                vartab,
                Instr::Set {
                    loc: *loc,
                    res: *var_no,
                    expr: rhs,
                },
            )
        }

        ast::YulExpression::SuffixAccess(_, member, suffix) => {
            match &**member {
                ast::YulExpression::SolidityLocalVariable(
                    _,
                    _,
                    Some(StorageLocation::Calldata(_)),
                    var_no,
                ) => match suffix {
                    YulSuffix::Offset => {
                        let rhs = rhs.cast(&lhs.ty(), ns);
                        cfg.add(
                            vartab,
                            Instr::Set {
                                loc: *loc,
                                res: *var_no,
                                expr: rhs,
                            },
                        );
                    }
                    YulSuffix::Length => {
                        unimplemented!("Assignment to calldata array suffix is not implemented");
                    }

                    _ => unreachable!(),
                },
                ast::YulExpression::SolidityLocalVariable(
                    _,
                    ty @ Type::ExternalFunction { .. },
                    _,
                    var_no,
                ) => {
                    let (member_no, casted_expr, member_ty) = match suffix {
                        YulSuffix::Selector => (
                            0,
                            rhs.cast(&Type::Bytes(ns.target.selector_length()), ns),
                            Type::Bytes(ns.target.selector_length()),
                        ),
                        YulSuffix::Address => {
                            (1, rhs.cast(&Type::Address(false), ns), Type::Address(false))
                        }
                        _ => unreachable!(),
                    };

                    let ptr = Expression::StructMember {
                        loc: *loc,
                        ty: Type::Ref(Box::new(member_ty)),
                        expr: Box::new(Expression::Variable {
                            loc: *loc,
                            ty: ty.clone(),
                            var_no: *var_no,
                        }),
                        member: member_no,
                    };

                    cfg.add(
                        vartab,
                        Instr::Store {
                            dest: ptr,
                            data: casted_expr,
                        },
                    );
                }

                ast::YulExpression::SolidityLocalVariable(
                    _,
                    _,
                    Some(StorageLocation::Storage(_)),
                    var_no,
                ) => {
                    // This assignment changes the value of a pointer to storage
                    if matches!(suffix, YulSuffix::Slot) {
                        let rhs = rhs.cast(&lhs.ty(), ns);
                        cfg.add(
                            vartab,
                            Instr::Set {
                                loc: *loc,
                                res: *var_no,
                                expr: rhs,
                            },
                        );
                    }
                }

                _ => unreachable!("There should not exist a suffix for the given expression"),
            }
        }

        ast::YulExpression::BoolLiteral { .. }
        | ast::YulExpression::NumberLiteral(..)
        | ast::YulExpression::StringLiteral(..)
        | ast::YulExpression::SolidityLocalVariable(..)
        | ast::YulExpression::StorageVariable(..)
        | ast::YulExpression::BuiltInCall(..)
        | ast::YulExpression::FunctionCall(..)
        | ast::YulExpression::ConstantVariable(..) => {
            unreachable!("Cannot assign to this expression");
        }
    }
}

/// Add an if statement to the CFG
fn process_if_block(
    cond: &ast::YulExpression,
    block: &ast::YulBlock,
    contract_no: usize,
    loops: &mut LoopScopes,
    ns: &Namespace,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    early_return: &Option<Instr>,
    opt: &Options,
) {
    let cond = expression(cond, contract_no, ns, vartab, cfg, opt);

    let bool_cond = if cond.ty() == Type::Bool {
        cond
    } else {
        Expression::NotEqual {
            loc: block.loc,
            left: Box::new(Expression::NumberLiteral {
                loc: pt::Loc::Codegen,
                ty: cond.ty(),
                value: BigInt::from_u8(0).unwrap(),
            }),
            right: Box::new(cond),
        }
    };

    let then = cfg.new_basic_block("then".to_string());
    let endif = cfg.new_basic_block("endif".to_string());

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: bool_cond,
            true_block: then,
            false_block: endif,
        },
    );

    cfg.set_basic_block(then);
    vartab.new_dirty_tracker();

    for stmt in &block.statements {
        statement(stmt, contract_no, loops, ns, cfg, vartab, early_return, opt);
    }

    if block.is_next_reachable() {
        cfg.add(vartab, Instr::Branch { block: endif });
    }

    cfg.set_phis(endif, vartab.pop_dirty_tracker());

    cfg.set_basic_block(endif);
}

/// Add the for statement to the CFG
fn process_for_block(
    loc: &pt::Loc,
    init_block: &ast::YulBlock,
    condition: &ast::YulExpression,
    post_block: &ast::YulBlock,
    execution_block: &ast::YulBlock,
    contract_no: usize,
    ns: &Namespace,
    loops: &mut LoopScopes,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    early_return: &Option<Instr>,
    opt: &Options,
) {
    for stmt in &init_block.statements {
        statement(stmt, contract_no, loops, ns, cfg, vartab, early_return, opt);
    }

    if !init_block.is_next_reachable() {
        return;
    }

    let cond_block = cfg.new_basic_block("cond".to_string());
    let next_block = cfg.new_basic_block("next".to_string());
    let body_block = cfg.new_basic_block("body".to_string());
    let end_block = cfg.new_basic_block("end_for".to_string());

    cfg.add(vartab, Instr::Branch { block: cond_block });
    cfg.set_basic_block(cond_block);

    let cond_expr = expression(condition, contract_no, ns, vartab, cfg, opt);

    let cond_expr = if cond_expr.ty() == Type::Bool {
        cond_expr
    } else {
        Expression::NotEqual {
            loc: *loc,
            left: Box::new(Expression::NumberLiteral {
                loc: pt::Loc::Codegen,
                ty: cond_expr.ty(),
                value: BigInt::from_u8(0).unwrap(),
            }),
            right: Box::new(cond_expr),
        }
    };

    cfg.add(
        vartab,
        Instr::BranchCond {
            cond: cond_expr,
            true_block: body_block,
            false_block: end_block,
        },
    );

    cfg.set_basic_block(body_block);
    loops.enter_scope(end_block, next_block);
    vartab.new_dirty_tracker();

    for stmt in &execution_block.statements {
        statement(stmt, contract_no, loops, ns, cfg, vartab, early_return, opt);
    }

    if execution_block.is_next_reachable() {
        cfg.add(vartab, Instr::Branch { block: next_block });
    }

    loops.leave_scope();

    cfg.set_basic_block(next_block);

    for stmt in &post_block.statements {
        statement(stmt, contract_no, loops, ns, cfg, vartab, early_return, opt);
    }

    if post_block.is_next_reachable() {
        cfg.add(vartab, Instr::Branch { block: cond_block });
    }

    cfg.set_basic_block(end_block);
    let set = vartab.pop_dirty_tracker();
    cfg.set_phis(next_block, set.clone());
    cfg.set_phis(end_block, set.clone());
    cfg.set_phis(cond_block, set);
}

/// Generate CFG code for a switch statement
fn switch(
    condition: &YulExpression,
    cases: &[CaseBlock],
    default: &Option<YulBlock>,
    loops: &mut LoopScopes,
    contract_no: usize,
    ns: &Namespace,
    vartab: &mut Vartable,
    cfg: &mut ControlFlowGraph,
    early_return: &Option<Instr>,
    opt: &Options,
) {
    let cond = expression(condition, contract_no, ns, vartab, cfg, opt);
    let end_switch = cfg.new_basic_block("end_switch".to_string());

    let current_block = cfg.current_block();

    vartab.new_dirty_tracker();
    let mut cases_cfg: Vec<(Expression, usize)> = Vec::with_capacity(cases.len());
    for (item_no, item) in cases.iter().enumerate() {
        let case_cond =
            expression(&item.condition, contract_no, ns, vartab, cfg, opt).cast(&cond.ty(), ns);
        let case_block = cfg.new_basic_block(format!("case_{item_no}"));
        cfg.set_basic_block(case_block);
        for stmt in &item.block.statements {
            statement(stmt, contract_no, loops, ns, cfg, vartab, early_return, opt);
        }
        if item.block.is_next_reachable() {
            cfg.add(vartab, Instr::Branch { block: end_switch });
        }
        cases_cfg.push((case_cond, case_block));
    }

    let default_block = if let Some(default_block) = default {
        let new_block = cfg.new_basic_block("default".to_string());
        cfg.set_basic_block(new_block);
        for stmt in &default_block.statements {
            statement(stmt, contract_no, loops, ns, cfg, vartab, early_return, opt);
        }
        if default_block.is_next_reachable() {
            cfg.add(vartab, Instr::Branch { block: end_switch });
        }
        new_block
    } else {
        end_switch
    };

    cfg.set_phis(end_switch, vartab.pop_dirty_tracker());

    cfg.set_basic_block(current_block);

    cfg.add(
        vartab,
        Instr::Switch {
            cond,
            cases: cases_cfg,
            default: default_block,
        },
    );

    cfg.set_basic_block(end_switch);
}
