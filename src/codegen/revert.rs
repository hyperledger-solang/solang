// SPDX-License-Identifier: Apache-2.0

use super::encoding::abi_encode;
use super::expression::expression;
use super::Options;
use super::{
    cfg::{ControlFlowGraph, Instr},
    vartable::Vartable,
};

use crate::codegen::Expression;
use crate::sema::{
    ast,
    ast::{FormatArg, Function, Namespace, Type},
    file::PathDisplay,
};
use crate::Target;
use num_bigint::BigInt;
use solang_parser::pt::{CodeLocation, Loc, Loc::Codegen};

/// This function encodes the arguments for the assert-failure instruction
/// and inserts it in the CFG.
pub(crate) fn assert_failure(
    loc: &Loc,
    arg: Option<Expression>,
    ns: &Namespace,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) {
    if arg.is_none() {
        cfg.add(vartab, Instr::AssertFailure { encoded_args: None });
        return;
    }

    let selector = 0x08c3_79a0u32;
    let selector = Expression::NumberLiteral {
        loc: Loc::Codegen,
        ty: Type::Bytes(4),
        value: BigInt::from(selector),
    };

    let args = vec![selector, arg.unwrap()];
    let (encoded_buffer, _) = abi_encode(loc, args, ns, vartab, cfg, false);

    cfg.add(
        vartab,
        Instr::AssertFailure {
            encoded_args: Some(encoded_buffer),
        },
    )
}

pub(super) fn expr_assert(
    cfg: &mut ControlFlowGraph,
    args: &ast::Expression,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
) -> Expression {
    let true_ = cfg.new_basic_block("noassert".to_owned());
    let false_ = cfg.new_basic_block("doassert".to_owned());
    let cond = expression(args, cfg, contract_no, func, ns, vartab, opt);
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_block: true_,
            false_block: false_,
        },
    );
    cfg.set_basic_block(false_);
    log_runtime_error(
        opt.log_runtime_errors,
        "assert failure",
        args.loc(),
        cfg,
        vartab,
        ns,
    );
    assert_failure(&Loc::Codegen, None, ns, cfg, vartab);
    cfg.set_basic_block(true_);
    Expression::Poison
}

pub(super) fn require(
    cfg: &mut ControlFlowGraph,
    args: &[ast::Expression],
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
    loc: Loc,
) -> Expression {
    let true_ = cfg.new_basic_block("noassert".to_owned());
    let false_ = cfg.new_basic_block("doassert".to_owned());
    let cond = expression(&args[0], cfg, contract_no, func, ns, vartab, opt);
    cfg.add(
        vartab,
        Instr::BranchCond {
            cond,
            true_block: true_,
            false_block: false_,
        },
    );
    cfg.set_basic_block(false_);
    let expr = args
        .get(1)
        .map(|s| expression(s, cfg, contract_no, func, ns, vartab, opt));
    match ns.target {
        // On Solana and Polkadot, print the reason, do not abi encode it
        Target::Solana | Target::Polkadot { .. } => {
            if opt.log_runtime_errors {
                if let Some(expr) = expr {
                    let prefix = b"runtime_error: ";
                    let error_string = format!(
                        " require condition failed in {},\n",
                        ns.loc_to_string(PathDisplay::Filename, &expr.loc())
                    );
                    let print_expr = Expression::FormatString {
                        loc: Loc::Codegen,
                        args: vec![
                            (
                                FormatArg::StringLiteral,
                                Expression::BytesLiteral {
                                    loc: Loc::Codegen,
                                    ty: Type::Bytes(prefix.len() as u8),
                                    value: prefix.to_vec(),
                                },
                            ),
                            (FormatArg::Default, expr),
                            (
                                FormatArg::StringLiteral,
                                Expression::BytesLiteral {
                                    loc: Loc::Codegen,
                                    ty: Type::Bytes(error_string.as_bytes().len() as u8),
                                    value: error_string.as_bytes().to_vec(),
                                },
                            ),
                        ],
                    };
                    cfg.add(vartab, Instr::Print { expr: print_expr });
                } else {
                    log_runtime_error(
                        opt.log_runtime_errors,
                        "require condition failed",
                        loc,
                        cfg,
                        vartab,
                        ns,
                    );
                }
            }
            assert_failure(&Loc::Codegen, None, ns, cfg, vartab);
        }
        _ => assert_failure(&Loc::Codegen, expr, ns, cfg, vartab),
    }
    cfg.set_basic_block(true_);
    Expression::Poison
}

pub(crate) fn log_runtime_error(
    report_error: bool,
    reason: &str,
    reason_loc: Loc,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    ns: &Namespace,
) {
    if report_error {
        let error_with_loc = error_msg_with_loc(ns, reason.to_string(), Some(reason_loc));
        let expr = string_to_expr(error_with_loc);
        cfg.add(vartab, Instr::Print { expr });
    }
}

pub(super) fn revert(
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
    loc: &Loc,
) {
    let expr = args
        .get(0)
        .map(|s| expression(s, cfg, contract_no, func, ns, vartab, opt));

    if opt.log_runtime_errors {
        if expr.is_some() {
            let prefix = b"runtime_error: ";
            let error_string = format!(
                " revert encountered in {},\n",
                ns.loc_to_string(PathDisplay::Filename, loc)
            );
            let print_expr = Expression::FormatString {
                loc: Codegen,
                args: vec![
                    (
                        FormatArg::StringLiteral,
                        Expression::BytesLiteral {
                            loc: Codegen,
                            ty: Type::Bytes(prefix.len() as u8),
                            value: prefix.to_vec(),
                        },
                    ),
                    (FormatArg::Default, expr.clone().unwrap()),
                    (
                        FormatArg::StringLiteral,
                        Expression::BytesLiteral {
                            loc: Codegen,
                            ty: Type::Bytes(error_string.as_bytes().len() as u8),
                            value: error_string.as_bytes().to_vec(),
                        },
                    ),
                ],
            };
            cfg.add(vartab, Instr::Print { expr: print_expr });
        } else {
            log_runtime_error(
                opt.log_runtime_errors,
                "revert encountered",
                *loc,
                cfg,
                vartab,
                ns,
            )
        }
    }

    assert_failure(&Codegen, expr, ns, cfg, vartab);
}

pub(crate) fn error_msg_with_loc(ns: &Namespace, error: String, loc: Option<Loc>) -> String {
    match &loc {
        Some(loc @ Loc::File(..)) => {
            let loc_from_file = ns.loc_to_string(PathDisplay::Filename, loc);
            format!("runtime_error: {error} in {loc_from_file},\n")
        }
        _ => error + ",\n",
    }
}

fn string_to_expr(string: String) -> Expression {
    Expression::FormatString {
        loc: Loc::Codegen,
        args: vec![(
            FormatArg::StringLiteral,
            Expression::BytesLiteral {
                loc: Loc::Codegen,
                ty: Type::Bytes(string.as_bytes().len() as u8),
                value: string.as_bytes().to_vec(),
            },
        )],
    }
}
