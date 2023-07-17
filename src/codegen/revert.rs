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
use num_traits::FromPrimitive;
use solang_parser::pt::{CodeLocation, Loc, Loc::Codegen};

/// Corresponds to the error types from the Solidity language.
///
/// Marked as non-exhaustive because Solidity may add more variants in the future.
#[non_exhaustive]
#[allow(unused)] // TODO: Implement custom errors
#[derive(Debug)]
pub(crate) enum ErrorSelector {
    /// Reverts with "empty error data"; stems from `revert()` or `require()` without string arguments.
    Empty,
    /// The `Error(string)` selector
    String,
    /// The `Panic(uint255)` selector
    Panic,
    /// The contract can define custom errors resulting in a custom selector
    Custom([u8; 4]),
}

impl Into<Expression> for ErrorSelector {
    fn into(self) -> Expression {
        match self {
            Self::Empty => unreachable!("empty return data can not be represented as Expression"),
            Self::String => Expression::NumberLiteral {
                loc: Codegen,
                ty: Type::Bytes(4),
                value: 0x08c379a0.into(),
            },
            Self::Panic => Expression::NumberLiteral {
                loc: Codegen,
                ty: Type::Bytes(4),
                value: 0x4e487b71.into(),
            },
            Self::Custom(bytes) => Expression::NumberLiteral {
                loc: Codegen,
                ty: Type::Bytes(4),
                value: u32::from_be_bytes(bytes).into(),
            },
        }
    }
}

/// Solidity `Panic` Codes. Source:
/// https://docs.soliditylang.org/en/v0.8.20/control-structures.html#panic-via-assert-and-error-via-require
#[allow(unused)]
#[non_exhaustive]
pub(crate) enum PanicCode {
    Generic = 0x00,
    AssertFailed = 0x01,
    MathOverflow = 0x11,
    DivisionByZero = 0x12,
    EnumCastOob = 0x21,
    StorageBytesEncodingIncorrect = 0x22,
    EmptyArrayPop = 0x31,
    ArrayIndexOob = 0x32,
    OutOfMemory = 0x41,
    InternalFunctionUninitialized = 0x51,
}

impl Into<BigInt> for PanicCode {
    fn into(self) -> BigInt {
        BigInt::from_isize(self as isize).expect("Panic codes can always be represented as BigInt")
    }
}

impl Into<Expression> for PanicCode {
    fn into(self) -> Expression {
        Expression::NumberLiteral {
            loc: Codegen,
            ty: Type::Uint(256),
            value: self.into(),
        }
    }
}

/// This function encodes the arguments for the assert-failure instruction
/// and inserts it in the CFG.
pub(super) fn assert_failure(
    loc: &Loc,
    arg: Option<Expression>,
    ns: &Namespace,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) {
    // On Solana, returning the encoded arguments has no effect
    if arg.is_none() || ns.target == Target::Solana {
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

#[cfg(test)]
mod tests {
    use num_bigint::BigInt;

    use crate::{
        codegen::{
            revert::{ErrorSelector, PanicCode},
            Expression,
        },
        sema::ast::Type,
    };

    #[test]
    fn panic_code_conversion() {
        assert_eq!(BigInt::from(0x00), PanicCode::Generic.into());
        assert_eq!(BigInt::from(0x01), PanicCode::AssertFailed.into());
        assert_eq!(BigInt::from(0x11), PanicCode::MathOverflow.into());
        assert_eq!(BigInt::from(0x12), PanicCode::DivisionByZero.into());
        assert_eq!(BigInt::from(0x21), PanicCode::EnumCastOob.into());
        assert_eq!(
            BigInt::from(0x22),
            PanicCode::StorageBytesEncodingIncorrect.into()
        );
        assert_eq!(BigInt::from(0x31), PanicCode::EmptyArrayPop.into());
        assert_eq!(BigInt::from(0x32), PanicCode::ArrayIndexOob.into());
        assert_eq!(BigInt::from(0x41), PanicCode::OutOfMemory.into());
        assert_eq!(
            BigInt::from(0x51),
            PanicCode::InternalFunctionUninitialized.into()
        );
    }

    #[test]
    fn function_selector_expression() {
        for (selector, expression) in [
            (0x08c379a0u32, ErrorSelector::String.into()),
            (0x4e487b71u32, ErrorSelector::Panic.into()),
            (
                0xdeadbeefu32,
                ErrorSelector::Custom([0xde, 0xad, 0xbe, 0xef]).into(),
            ),
        ] {
            match expression {
                Expression::NumberLiteral { ty, value, .. } => {
                    assert_eq!(ty, Type::Bytes(4));
                    assert_eq!(value, selector.into());
                }
                _ => panic!("invalid selector expression generated for {:?}", expression),
            }
        }
    }
}
