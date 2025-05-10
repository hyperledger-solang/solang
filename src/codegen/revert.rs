// SPDX-License-Identifier: Apache-2.0

//! Releated to code that ultimately compiles to the target
//! equivalent instruction of EVM revert (0xfd).

use super::encoding::{abi_encode, create_encoder};
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
use num_bigint::{BigInt, Sign};
use parse_display::Display;
use solang_parser::pt::{CodeLocation, Loc, Loc::Codegen};
use tiny_keccak::{Hasher, Keccak};

/// Signature of `Keccak256('Error(string)')[:4]`
pub(crate) const ERROR_SELECTOR: [u8; 4] = [0x08, 0xc3, 0x79, 0xa0];
/// Signature of `Keccak256('Panic(uint256)')[:4]`
pub(crate) const PANIC_SELECTOR: [u8; 4] = [0x4e, 0x48, 0x7b, 0x71];

/// Corresponds to the error types from the Solidity language.
///
/// Marked as non-exhaustive because Solidity may add more variants in the future.
#[non_exhaustive]
#[derive(Debug, PartialEq, Clone)]
pub enum SolidityError {
    /// Reverts with "empty error data"; stems from `revert()` or `require()` without string arguments.
    Empty,
    /// The `Error(string)` selector
    String(Expression),
    /// The `Panic(uint256)` selector
    Panic(PanicCode),
    /// User defined errors
    Custom {
        error_no: usize,
        exprs: Vec<Expression>,
    },
}

impl SolidityError {
    /// Return the selector expression of the error.
    pub fn selector_expression(&self, ns: &Namespace) -> Expression {
        Expression::NumberLiteral {
            loc: Codegen,
            ty: Type::Bytes(4),
            value: BigInt::from_bytes_be(Sign::Plus, &self.selector(ns)),
        }
    }

    /// Return the selector of the error.
    pub fn selector(&self, ns: &Namespace) -> [u8; 4] {
        match self {
            Self::Empty => unreachable!("empty return data has no selector"),
            Self::String(_) => ERROR_SELECTOR,
            Self::Panic(_) => PANIC_SELECTOR,
            Self::Custom { error_no, .. } => {
                let mut buf = [0u8; 32];
                let mut hasher = Keccak::v256();
                let signature =
                    ns.signature(&ns.errors[*error_no].name, &ns.errors[*error_no].fields);
                hasher.update(signature.as_bytes());
                hasher.finalize(&mut buf);
                [buf[0], buf[1], buf[2], buf[3]]
            }
        }
    }

    /// ABI encode the selector and any error data.
    ///
    /// Returns `None` if the data can't be ABI encoded.
    pub(super) fn abi_encode(
        &self,
        loc: &Loc,
        ns: &Namespace,
        vartab: &mut Vartable,
        cfg: &mut ControlFlowGraph,
    ) -> Option<Expression> {
        match self {
            Self::Empty => None,
            Self::String(expr) => {
                let args = vec![self.selector_expression(ns), expr.clone()];
                create_encoder(ns, false)
                    .const_encode(&args)
                    .map(|bytes| {
                        let size = Expression::NumberLiteral {
                            loc: Codegen,
                            ty: Type::Uint(32),
                            value: bytes.len().into(),
                        };
                        Expression::AllocDynamicBytes {
                            loc: Codegen,
                            ty: Type::Slice(Type::Bytes(1).into()),
                            size: size.into(),
                            initializer: bytes.into(),
                        }
                    })
                    .or_else(|| abi_encode(loc, args, ns, vartab, cfg, false).0.into())
            }
            Self::Custom { exprs, .. } => {
                let mut args = exprs.to_owned();
                args.insert(0, self.selector_expression(ns));
                create_encoder(ns, false)
                    .const_encode(&args)
                    .map(|bytes| {
                        let size = Expression::NumberLiteral {
                            loc: Codegen,
                            ty: Type::Uint(32),
                            value: bytes.len().into(),
                        };
                        Expression::AllocDynamicBytes {
                            loc: Codegen,
                            ty: Type::Slice(Type::Bytes(1).into()),
                            size: size.into(),
                            initializer: bytes.into(),
                        }
                    })
                    .or_else(|| abi_encode(loc, args, ns, vartab, cfg, false).0.into())
            }
            Self::Panic(code) => {
                let code = Expression::NumberLiteral {
                    loc: Codegen,
                    ty: Type::Uint(256),
                    value: (*code as u8).into(),
                };
                create_encoder(ns, false)
                    .const_encode(&[self.selector_expression(ns), code])
                    .map(|bytes| {
                        let size = Expression::NumberLiteral {
                            loc: Codegen,
                            ty: Type::Uint(32),
                            value: bytes.len().into(),
                        };
                        Expression::AllocDynamicBytes {
                            loc: Codegen,
                            ty: Type::Slice(Type::Bytes(1).into()),
                            size: size.into(),
                            initializer: bytes.into(),
                        }
                    })
            }
        }
    }
}

/// Solidity `Panic` Codes. Source:
/// https://docs.soliditylang.org/en/v0.8.20/control-structures.html#panic-via-assert-and-error-via-require
///
/// FIXME: Currently, not all panic variants are wired up yet in Solang:
/// * EnumCastOob
/// * StorageBytesEncodingIncorrect
/// * OutOfMemory
///
/// Tracking issue: <https://github.com/hyperledger-solang/solang/issues/1477>
#[derive(Display, Debug, PartialEq, Clone, Copy)]
#[non_exhaustive]
#[repr(u8)]
pub enum PanicCode {
    Generic = 0x00,
    Assertion = 0x01,
    MathOverflow = 0x11,
    DivisionByZero = 0x12,
    EnumCastOob = 0x21,
    StorageBytesEncodingIncorrect = 0x22,
    EmptyArrayPop = 0x31,
    ArrayIndexOob = 0x32,
    OutOfMemory = 0x41,
    InternalFunctionUninitialized = 0x51,
}

/// This function encodes the arguments for the assert-failure instruction
/// and inserts it in the CFG.
pub(super) fn assert_failure(
    loc: &Loc,
    error: SolidityError,
    ns: &Namespace,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
) {
    // On Solana, returning the encoded arguments has no effect
    if ns.target == Target::Solana {
        cfg.add(vartab, Instr::AssertFailure { encoded_args: None });
        return;
    }

    let encoded_args = error.abi_encode(loc, ns, vartab, cfg);
    cfg.add(vartab, Instr::AssertFailure { encoded_args })
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
    let error = SolidityError::Panic(PanicCode::Assertion);
    assert_failure(&Codegen, error, ns, cfg, vartab);
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

    // On Solana and Polkadot, print the reason
    if opt.log_runtime_errors && (ns.target == Target::Solana || ns.target.is_polkadot()) {
        if let Some(expr) = expr.clone() {
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
                            ty: Type::Bytes(error_string.len() as u8),
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

    let error = expr
        .map(SolidityError::String)
        .unwrap_or(SolidityError::Empty);
    assert_failure(&Codegen, error, ns, cfg, vartab);

    cfg.set_basic_block(true_);
    Expression::Poison
}

pub(super) fn revert(
    args: &[ast::Expression],
    error_no: &Option<usize>,
    cfg: &mut ControlFlowGraph,
    contract_no: usize,
    func: Option<&Function>,
    ns: &Namespace,
    vartab: &mut Vartable,
    opt: &Options,
    loc: &Loc,
) {
    let exprs = args
        .iter()
        .map(|s| expression(s, cfg, contract_no, func, ns, vartab, opt))
        .collect::<Vec<_>>();

    if opt.log_runtime_errors {
        match (error_no, exprs.first()) {
            // In the case of Error(string), we can print the reason
            (None, Some(expr)) => {
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
                        (FormatArg::Default, expr.clone()),
                        (
                            FormatArg::StringLiteral,
                            Expression::BytesLiteral {
                                loc: Codegen,
                                ty: Type::Bytes(error_string.len() as u8),
                                value: error_string.as_bytes().to_vec(),
                            },
                        ),
                    ],
                };
                cfg.add(vartab, Instr::Print { expr: print_expr });
            }
            // Else: Not all fields might be formattable, just print the error type
            _ => {
                let error_ty = error_no
                    .map(|n| ns.errors[n].name.as_str())
                    .unwrap_or("unspecified");
                let reason = format!("{} revert encountered", error_ty);
                log_runtime_error(opt.log_runtime_errors, &reason, *loc, cfg, vartab, ns);
            }
        }
    }

    let error = match (*error_no, exprs.first()) {
        // Having an error number requires a custom error
        (Some(error_no), _) => SolidityError::Custom { error_no, exprs },
        // No error number but an expression requires Error(String)
        (None, Some(expr)) => SolidityError::String(expr.clone()),
        // No error number and no data means just "revert();" without any reason
        (None, None) => SolidityError::Empty,
    };
    assert_failure(&Codegen, error, ns, cfg, vartab);
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

pub(crate) fn error_msg_with_loc(ns: &Namespace, error: String, loc: Option<Loc>) -> String {
    match &loc {
        Some(loc @ Loc::File(..)) => {
            let loc_from_file = ns.loc_to_string(PathDisplay::Filename, loc);
            format!("runtime_error: {error} in {loc_from_file},\n")
        }
        _ => error + ",\n",
    }
}

pub(super) fn string_to_expr(string: String) -> Expression {
    Expression::FormatString {
        loc: Loc::Codegen,
        args: vec![(
            FormatArg::StringLiteral,
            Expression::BytesLiteral {
                loc: Loc::Codegen,
                ty: Type::Bytes(string.len() as u8),
                value: string.as_bytes().to_vec(),
            },
        )],
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        codegen::{
            revert::{PanicCode, SolidityError, ERROR_SELECTOR, PANIC_SELECTOR},
            Expression,
        },
        sema::ast::{ErrorDecl, Namespace, Parameter, Type},
        Target,
    };

    #[test]
    fn panic_code_as_byte() {
        assert_eq!(0x00, PanicCode::Generic as u8);
        assert_eq!(0x01, PanicCode::Assertion as u8);
        assert_eq!(0x11, PanicCode::MathOverflow as u8);
        assert_eq!(0x12, PanicCode::DivisionByZero as u8);
        assert_eq!(0x21, PanicCode::EnumCastOob as u8);
        assert_eq!(0x22, PanicCode::StorageBytesEncodingIncorrect as u8);
        assert_eq!(0x31, PanicCode::EmptyArrayPop as u8);
        assert_eq!(0x32, PanicCode::ArrayIndexOob as u8);
        assert_eq!(0x41, PanicCode::OutOfMemory as u8);
        assert_eq!(0x51, PanicCode::InternalFunctionUninitialized as u8);
    }

    #[test]
    fn default_error_selector_expression() {
        let ns = Namespace::new(Target::default_polkadot());
        assert_eq!(
            ERROR_SELECTOR,
            SolidityError::String(Expression::Poison).selector(&ns),
        );
        assert_eq!(
            PANIC_SELECTOR,
            SolidityError::Panic(PanicCode::Generic).selector(&ns),
        );
    }

    /// Error selector calculation uses the same signature algorithm used for message selectors.
    /// Tests the error selector calculation to be correct against two examples:
    /// - `error ERC20InsufficientBalance(address sender, uint256 balance, uint256 needed);`
    /// - `error Unauthorized();`
    #[test]
    fn custom_error_selector_expression() {
        let mut ns = Namespace::new(Target::default_polkadot());
        ns.errors = vec![
            ErrorDecl {
                name: "Unauthorized".to_string(),
                ..Default::default()
            },
            ErrorDecl {
                name: "ERC20InsufficientBalance".to_string(),
                fields: vec![
                    Parameter::new_default(Type::Address(false)),
                    Parameter::new_default(Type::Uint(256)),
                    Parameter::new_default(Type::Uint(256)),
                ],
                ..Default::default()
            },
        ];

        let exprs = vec![Expression::Poison];
        let expected_selector = SolidityError::Custom { error_no: 0, exprs }.selector(&ns);
        assert_eq!([0x82, 0xb4, 0x29, 0x00], expected_selector);

        let exprs = vec![Expression::Poison];
        let expected_selector = SolidityError::Custom { error_no: 1, exprs }.selector(&ns);
        assert_eq!([0xe4, 0x50, 0xd3, 0x8c], expected_selector);
    }
}
