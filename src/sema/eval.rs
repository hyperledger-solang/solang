// SPDX-License-Identifier: Apache-2.0

use super::ast::{Diagnostic, Expression, Namespace, Type};
use num_bigint::BigInt;
use num_bigint::Sign;
use num_rational::BigRational;
use num_traits::One;
use num_traits::ToPrimitive;
use num_traits::Zero;
use solang_parser::pt;
use solang_parser::pt::{CodeLocation, Loc};
use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Shl, Shr, Sub};

/// Resolve an expression where a compile-time constant is expected
pub fn eval_const_number(
    expr: &Expression,
    ns: &Namespace,
) -> Result<(pt::Loc, BigInt), Diagnostic> {
    match expr {
        Expression::Add(loc, _, _, l, r) => Ok((
            *loc,
            eval_const_number(l, ns)?.1 + eval_const_number(r, ns)?.1,
        )),
        Expression::Subtract(loc, _, _, l, r) => Ok((
            *loc,
            eval_const_number(l, ns)?.1 - eval_const_number(r, ns)?.1,
        )),
        Expression::Multiply(loc, _, _, l, r) => Ok((
            *loc,
            eval_const_number(l, ns)?.1 * eval_const_number(r, ns)?.1,
        )),
        Expression::Divide(loc, _, l, r) => {
            let divisor = eval_const_number(r, ns)?.1;

            if divisor.is_zero() {
                Err(Diagnostic::error(*loc, "divide by zero".to_string()))
            } else {
                Ok((*loc, eval_const_number(l, ns)?.1 / divisor))
            }
        }
        Expression::Modulo(loc, _, l, r) => {
            let divisor = eval_const_number(r, ns)?.1;

            if divisor.is_zero() {
                Err(Diagnostic::error(*loc, "divide by zero".to_string()))
            } else {
                Ok((*loc, eval_const_number(l, ns)?.1 % divisor))
            }
        }
        Expression::BitwiseAnd(loc, _, l, r) => Ok((
            *loc,
            eval_const_number(l, ns)?.1 & eval_const_number(r, ns)?.1,
        )),
        Expression::BitwiseOr(loc, _, l, r) => Ok((
            *loc,
            eval_const_number(l, ns)?.1 | eval_const_number(r, ns)?.1,
        )),
        Expression::BitwiseXor(loc, _, l, r) => Ok((
            *loc,
            eval_const_number(l, ns)?.1 ^ eval_const_number(r, ns)?.1,
        )),
        Expression::Power(loc, _, _, base, exp) => {
            let b = eval_const_number(base, ns)?.1;
            let mut e = eval_const_number(exp, ns)?.1;

            if e.sign() == Sign::Minus {
                Err(Diagnostic::error(
                    expr.loc(),
                    "power cannot take negative number as exponent".to_string(),
                ))
            } else if e.sign() == Sign::NoSign {
                Ok((*loc, BigInt::one()))
            } else {
                let mut res = b.clone();
                e -= BigInt::one();
                while e.sign() == Sign::Plus {
                    res *= b.clone();
                    e -= BigInt::one();
                }
                Ok((*loc, res))
            }
        }
        Expression::ShiftLeft(loc, _, left, right) => {
            let l = eval_const_number(left, ns)?.1;
            let r = eval_const_number(right, ns)?.1;
            let r = match r.to_usize() {
                Some(r) => r,
                None => {
                    return Err(Diagnostic::error(
                        expr.loc(),
                        format!("cannot left shift by {}", r),
                    ));
                }
            };
            Ok((*loc, l << r))
        }
        Expression::ShiftRight(loc, _, left, right, _) => {
            let l = eval_const_number(left, ns)?.1;
            let r = eval_const_number(right, ns)?.1;
            let r = match r.to_usize() {
                Some(r) => r,
                None => {
                    return Err(Diagnostic::error(
                        expr.loc(),
                        format!("cannot right shift by {}", r),
                    ));
                }
            };
            Ok((*loc, l >> r))
        }
        Expression::NumberLiteral(loc, _, n) => Ok((*loc, n.clone())),
        Expression::ZeroExt(loc, _, n) => Ok((*loc, eval_const_number(n, ns)?.1)),
        Expression::SignExt(loc, _, n) => Ok((*loc, eval_const_number(n, ns)?.1)),
        Expression::Cast(loc, _, n) => Ok((*loc, eval_const_number(n, ns)?.1)),
        Expression::Not(loc, n) => Ok((*loc, !eval_const_number(n, ns)?.1)),
        Expression::Complement(loc, _, n) => Ok((*loc, !eval_const_number(n, ns)?.1)),
        Expression::UnaryMinus(loc, _, n) => Ok((*loc, -eval_const_number(n, ns)?.1)),
        Expression::ConstantVariable(_, _, Some(contract_no), var_no) => {
            let expr = ns.contracts[*contract_no].variables[*var_no]
                .initializer
                .as_ref()
                .unwrap()
                .clone();

            eval_const_number(&expr, ns)
        }
        Expression::ConstantVariable(_, _, None, var_no) => {
            let expr = ns.constants[*var_no].initializer.as_ref().unwrap().clone();

            eval_const_number(&expr, ns)
        }
        _ => Err(Diagnostic::error(
            expr.loc(),
            "expression not allowed in constant number expression".to_string(),
        )),
    }
}

/// Resolve an expression where a compile-time constant(rational) is expected
pub fn eval_const_rational(
    expr: &Expression,
    ns: &Namespace,
) -> Result<(pt::Loc, BigRational), Diagnostic> {
    match expr {
        Expression::Add(loc, _, _, l, r) => Ok((
            *loc,
            eval_const_rational(l, ns)?.1 + eval_const_rational(r, ns)?.1,
        )),
        Expression::Subtract(loc, _, _, l, r) => Ok((
            *loc,
            eval_const_rational(l, ns)?.1 - eval_const_rational(r, ns)?.1,
        )),
        Expression::Multiply(loc, _, _, l, r) => Ok((
            *loc,
            eval_const_rational(l, ns)?.1 * eval_const_rational(r, ns)?.1,
        )),
        Expression::Divide(loc, _, l, r) => {
            let divisor = eval_const_rational(r, ns)?.1;

            if divisor.is_zero() {
                Err(Diagnostic::error(*loc, "divide by zero".to_string()))
            } else {
                Ok((*loc, eval_const_rational(l, ns)?.1 / divisor))
            }
        }
        Expression::Modulo(loc, _, l, r) => {
            let divisor = eval_const_rational(r, ns)?.1;

            if divisor.is_zero() {
                Err(Diagnostic::error(*loc, "divide by zero".to_string()))
            } else {
                Ok((*loc, eval_const_rational(l, ns)?.1 % divisor))
            }
        }
        Expression::NumberLiteral(loc, _, n) => Ok((*loc, BigRational::from_integer(n.clone()))),
        Expression::RationalNumberLiteral(loc, _, n) => Ok((*loc, n.clone())),
        Expression::Cast(loc, _, n) => Ok((*loc, eval_const_rational(n, ns)?.1)),
        Expression::UnaryMinus(loc, _, n) => Ok((*loc, -eval_const_rational(n, ns)?.1)),
        Expression::ConstantVariable(_, _, Some(contract_no), var_no) => {
            let expr = ns.contracts[*contract_no].variables[*var_no]
                .initializer
                .as_ref()
                .unwrap()
                .clone();

            eval_const_rational(&expr, ns)
        }
        Expression::ConstantVariable(_, _, None, var_no) => {
            let expr = ns.constants[*var_no].initializer.as_ref().unwrap().clone();

            eval_const_rational(&expr, ns)
        }
        _ => Err(Diagnostic::error(
            expr.loc(),
            "expression not allowed in constant rational number expression".to_string(),
        )),
    }
}

/// Function that recurses the expression and folds number literals by calling 'eval_constants_in_expression'.
/// If the expression is an arithmetic operation of two number literals, overflow_check() will be called on the result.
pub(super) fn check_term_for_constant_overflow(expr: &Expression, ns: &mut Namespace) -> bool {
    match expr {
        Expression::Add(..)
        | Expression::Subtract(..)
        | Expression::Multiply(..)
        | Expression::Divide(..)
        | Expression::Modulo(..)
        | Expression::Power(..)
        | Expression::ShiftLeft(..)
        | Expression::ShiftRight(..)
        | Expression::BitwiseAnd(..)
        | Expression::BitwiseOr(..)
        | Expression::BitwiseXor(..)
        | Expression::NumberLiteral(..) => match eval_constants_in_expression(expr, ns) {
            (Some(Expression::NumberLiteral(loc, ty, result)), _) => {
                if let Some(diagnostic) = overflow_check(&result, &ty, &loc) {
                    ns.diagnostics.push(diagnostic);
                }

                return false;
            }
            (None, false) => {
                return false;
            }
            _ => {}
        },
        _ => {}
    }

    true
}

/// This function recursively folds number literals in a given expression.
/// It returns an Option<Expression> which is the result of the folding if the operands are number literals, and a boolean flag that is set to false if the recursion should stop.
fn eval_constants_in_expression(
    expr: &Expression,
    ns: &mut Namespace,
) -> (Option<Expression>, bool) {
    match expr {
        Expression::Add(loc, ty, _, left, right) => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (left, right)
            {
                (
                    Some(Expression::NumberLiteral(*loc, ty.clone(), left.add(right))),
                    true,
                )
            } else {
                (None, true)
            }
        }
        Expression::Subtract(loc, ty, _, left, right) => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (&left, &right)
            {
                (
                    Some(Expression::NumberLiteral(*loc, ty.clone(), left.sub(right))),
                    true,
                )
            } else {
                (None, true)
            }
        }

        Expression::Multiply(loc, ty, _, left, right) => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (&left, &right)
            {
                (
                    Some(Expression::NumberLiteral(*loc, ty.clone(), left.mul(right))),
                    true,
                )
            } else {
                (None, true)
            }
        }
        Expression::Divide(loc, ty, left, right) => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (&left, &right)
            {
                if right.is_zero() {
                    ns.diagnostics
                        .push(Diagnostic::error(*loc, "divide by zero".to_string()));
                    (None, false)
                } else {
                    (
                        Some(Expression::NumberLiteral(*loc, ty.clone(), left.div(right))),
                        true,
                    )
                }
            } else {
                (None, true)
            }
        }

        Expression::Modulo(loc, ty, left, right) => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (&left, &right)
            {
                if right.is_zero() {
                    ns.diagnostics
                        .push(Diagnostic::error(*loc, "divide by zero".to_string()));
                    (None, false)
                } else {
                    (
                        Some(Expression::NumberLiteral(*loc, ty.clone(), left % right)),
                        true,
                    )
                }
            } else {
                (None, true)
            }
        }
        Expression::Power(loc, ty, _, left, right) => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(right_loc, _, right)),
            ) = (&left, &right)
            {
                if overflow_check(right, &Type::Uint(16), right_loc).is_some() {
                    ns.diagnostics.push(Diagnostic::error(
                        *right_loc,
                        format!("power by {} is not possible", right),
                    ));
                    (None, false)
                } else {
                    (
                        Some(Expression::NumberLiteral(
                            *loc,
                            ty.clone(),
                            left.pow(right.to_u16().unwrap().into()),
                        )),
                        true,
                    )
                }
            } else {
                (None, true)
            }
        }
        Expression::ShiftLeft(loc, ty, left, right) => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(right_loc, _, right)),
            ) = (&left, &right)
            {
                if overflow_check(right, &Type::Uint(64), right_loc).is_some() {
                    ns.diagnostics.push(Diagnostic::error(
                        *right_loc,
                        format!("left shift by {} is not possible", right),
                    ));
                    (None, false)
                } else {
                    if right >= &BigInt::from(left.bits()) {
                        ns.diagnostics.push(Diagnostic::warning(
                            *right_loc,
                            format!("left shift by {} may overflow the final result", right),
                        ));
                    }

                    (
                        Some(Expression::NumberLiteral(
                            *loc,
                            ty.clone(),
                            left.shl(right.to_u64().unwrap()),
                        )),
                        true,
                    )
                }
            } else {
                (None, true)
            }
        }

        Expression::ShiftRight(loc, ty, left, right, _) => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(right_loc, _, right)),
            ) = (&left, &right)
            {
                if overflow_check(right, &Type::Uint(64), right_loc).is_some() {
                    ns.diagnostics.push(Diagnostic::error(
                        *right_loc,
                        format!("right shift by {} is not possible", right),
                    ));
                    (None, false)
                } else {
                    (
                        Some(Expression::NumberLiteral(
                            *loc,
                            ty.clone(),
                            left.shr(right.to_u64().unwrap()),
                        )),
                        true,
                    )
                }
            } else {
                (None, true)
            }
        }
        Expression::BitwiseAnd(loc, ty, left, right) => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (&left, &right)
            {
                (
                    Some(Expression::NumberLiteral(
                        *loc,
                        ty.clone(),
                        left.bitand(right),
                    )),
                    true,
                )
            } else {
                (None, true)
            }
        }
        Expression::BitwiseOr(loc, ty, left, right) => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (&left, &right)
            {
                (
                    Some(Expression::NumberLiteral(
                        *loc,
                        ty.clone(),
                        left.bitor(right),
                    )),
                    true,
                )
            } else {
                (None, true)
            }
        }
        Expression::BitwiseXor(loc, ty, left, right) => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral(_, _, left)),
                Some(Expression::NumberLiteral(_, _, right)),
            ) = (&left, &right)
            {
                (
                    Some(Expression::NumberLiteral(
                        *loc,
                        ty.clone(),
                        left.bitxor(right),
                    )),
                    true,
                )
            } else {
                (None, true)
            }
        }
        Expression::ZeroExt(loc, ty, expr) => {
            let expr = eval_constants_in_expression(expr, ns).0;
            if let Some(Expression::NumberLiteral(_, _, n)) = expr {
                (Some(Expression::NumberLiteral(*loc, ty.clone(), n)), true)
            } else {
                (None, true)
            }
        }
        Expression::SignExt(loc, ty, expr) => {
            let expr = eval_constants_in_expression(expr, ns).0;
            if let Some(Expression::NumberLiteral(_, _, n)) = expr {
                (Some(Expression::NumberLiteral(*loc, ty.clone(), n)), true)
            } else {
                (None, true)
            }
        }
        Expression::NumberLiteral(..) => (Some(expr.clone()), true),
        _ => (None, true),
    }
}

/// Function that takes a BigInt and an expected type. If the number of bits in the type required to represent the BigInt is not suffiecient, it will return a diagnostic.
fn overflow_check(result: &BigInt, ty: &Type, loc: &Loc) -> Option<Diagnostic> {
    if let Type::Uint(bits) = ty {
        // If the result sign is minus, throw an error.
        if let Sign::Minus = result.sign() {
            return Some(Diagnostic::error(
                *loc,
            format!( "negative value {} does not fit into type uint{}. Cannot implicitly convert signed literal to unsigned type.",result,ty.get_type_size()),
            ));
        }

        // If bits of the result is more than bits of the type, throw and error.
        if result.bits() > *bits as u64 {
            return Some(Diagnostic::error(
                *loc,
                format!(
                    "value {} does not fit into type uint{}.",
                    result,
                    ty.get_type_size(),
                ),
            ));
        }
    }

    if let Type::Int(bits) = ty {
        // If number of bits is more than what the type can hold. BigInt.bits() is not used here since it disregards the sign.
        if result.to_signed_bytes_be().len() * 8 > (*bits as usize) {
            return Some(Diagnostic::error(
                *loc,
                format!(
                    "value {} does not fit into type int{}.",
                    result,
                    ty.get_type_size(),
                ),
            ));
        }
    }
    None
}
