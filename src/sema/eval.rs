// SPDX-License-Identifier: Apache-2.0

use super::{
    ast::{Diagnostic, Expression, Namespace, Type},
    diagnostics::Diagnostics,
};
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
    diagnostics: &mut Diagnostics,
) -> Result<(pt::Loc, BigInt), ()> {
    match expr {
        Expression::Add {
            loc, left, right, ..
        } => Ok((
            *loc,
            eval_const_number(left, ns, diagnostics)?.1
                + eval_const_number(right, ns, diagnostics)?.1,
        )),
        Expression::Subtract {
            loc, left, right, ..
        } => Ok((
            *loc,
            eval_const_number(left, ns, diagnostics)?.1
                - eval_const_number(right, ns, diagnostics)?.1,
        )),
        Expression::Multiply {
            loc, left, right, ..
        } => Ok((
            *loc,
            eval_const_number(left, ns, diagnostics)?.1
                * eval_const_number(right, ns, diagnostics)?.1,
        )),
        Expression::Divide {
            loc, left, right, ..
        } => {
            let divisor = eval_const_number(right, ns, diagnostics)?.1;

            if divisor.is_zero() {
                diagnostics.push(Diagnostic::error(*loc, "divide by zero".to_string()));

                Err(())
            } else {
                Ok((*loc, eval_const_number(left, ns, diagnostics)?.1 / divisor))
            }
        }
        Expression::Modulo {
            loc, left, right, ..
        } => {
            let divisor = eval_const_number(right, ns, diagnostics)?.1;

            if divisor.is_zero() {
                diagnostics.push(Diagnostic::error(*loc, "divide by zero".to_string()));

                Err(())
            } else {
                Ok((*loc, eval_const_number(left, ns, diagnostics)?.1 % divisor))
            }
        }
        Expression::BitwiseAnd {
            loc, left, right, ..
        } => Ok((
            *loc,
            eval_const_number(left, ns, diagnostics)?.1
                & eval_const_number(right, ns, diagnostics)?.1,
        )),
        Expression::BitwiseOr {
            loc, left, right, ..
        } => Ok((
            *loc,
            eval_const_number(left, ns, diagnostics)?.1
                | eval_const_number(right, ns, diagnostics)?.1,
        )),
        Expression::BitwiseXor {
            loc, left, right, ..
        } => Ok((
            *loc,
            eval_const_number(left, ns, diagnostics)?.1
                ^ eval_const_number(right, ns, diagnostics)?.1,
        )),
        Expression::Power { loc, base, exp, .. } => {
            let b = eval_const_number(base, ns, diagnostics)?.1;
            let mut e = eval_const_number(exp, ns, diagnostics)?.1;

            if e.sign() == Sign::Minus {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "power cannot take negative number as exponent".to_string(),
                ));

                Err(())
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
        Expression::ShiftLeft {
            loc, left, right, ..
        } => {
            let l = eval_const_number(left, ns, diagnostics)?.1;
            let r = eval_const_number(right, ns, diagnostics)?.1;
            let r = match r.to_usize() {
                Some(r) => r,
                None => {
                    diagnostics.push(Diagnostic::error(*loc, format!("cannot left shift by {r}")));

                    return Err(());
                }
            };
            Ok((*loc, l << r))
        }
        Expression::ShiftRight {
            loc, left, right, ..
        } => {
            let l = eval_const_number(left, ns, diagnostics)?.1;
            let r = eval_const_number(right, ns, diagnostics)?.1;
            let r = match r.to_usize() {
                Some(r) => r,
                None => {
                    diagnostics.push(Diagnostic::error(*loc, format!("right left shift by {r}")));

                    return Err(());
                }
            };
            Ok((*loc, l >> r))
        }
        Expression::NumberLiteral { loc, value, .. } => Ok((*loc, value.clone())),
        Expression::ZeroExt { loc, expr, .. } => {
            Ok((*loc, eval_const_number(expr, ns, diagnostics)?.1))
        }
        Expression::SignExt { loc, expr, .. } => {
            Ok((*loc, eval_const_number(expr, ns, diagnostics)?.1))
        }
        Expression::Cast { loc, expr, .. } => {
            Ok((*loc, eval_const_number(expr, ns, diagnostics)?.1))
        }
        Expression::Not { loc, expr: n } => Ok((*loc, !eval_const_number(n, ns, diagnostics)?.1)),
        Expression::BitwiseNot { loc, expr, .. } => {
            Ok((*loc, !eval_const_number(expr, ns, diagnostics)?.1))
        }
        Expression::Negate { loc, expr, .. } => {
            Ok((*loc, -eval_const_number(expr, ns, diagnostics)?.1))
        }
        Expression::ConstantVariable {
            contract_no: Some(contract_no),
            var_no,
            ..
        } => {
            let var = &ns.contracts[*contract_no].variables[*var_no];

            if let Some(init) = &var.initializer {
                eval_const_number(init, ns, diagnostics)
            } else {
                // we should have errored about this already
                Err(())
            }
        }
        Expression::ConstantVariable {
            contract_no: None,
            var_no,
            ..
        } => {
            let var = &ns.constants[*var_no];

            if let Some(init) = &var.initializer {
                eval_const_number(init, ns, diagnostics)
            } else {
                // we should have errored about this already
                Err(())
            }
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                expr.loc(),
                "expression not allowed in constant number expression".to_string(),
            ));

            Err(())
        }
    }
}

/// Resolve an expression where a compile-time constant(rational) is expected
pub fn eval_const_rational(
    expr: &Expression,
    ns: &Namespace,
) -> Result<(pt::Loc, BigRational), Diagnostic> {
    match expr {
        Expression::Add {
            loc, left, right, ..
        } => Ok((
            *loc,
            eval_const_rational(left, ns)?.1 + eval_const_rational(right, ns)?.1,
        )),
        Expression::Subtract {
            loc, left, right, ..
        } => Ok((
            *loc,
            eval_const_rational(left, ns)?.1 - eval_const_rational(right, ns)?.1,
        )),
        Expression::Multiply {
            loc,
            left: l,
            right: r,
            ..
        } => Ok((
            *loc,
            eval_const_rational(l, ns)?.1 * eval_const_rational(r, ns)?.1,
        )),
        Expression::Divide {
            loc, left, right, ..
        } => {
            let divisor = eval_const_rational(right, ns)?.1;

            if divisor.is_zero() {
                Err(Diagnostic::error(*loc, "divide by zero".to_string()))
            } else {
                Ok((*loc, eval_const_rational(left, ns)?.1 / divisor))
            }
        }
        Expression::Modulo {
            loc,
            left: l,
            right: r,
            ..
        } => {
            let divisor = eval_const_rational(r, ns)?.1;

            if divisor.is_zero() {
                Err(Diagnostic::error(*loc, "divide by zero".to_string()))
            } else {
                Ok((*loc, eval_const_rational(l, ns)?.1 % divisor))
            }
        }
        Expression::NumberLiteral { loc, value, .. } => {
            Ok((*loc, BigRational::from_integer(value.clone())))
        }
        Expression::RationalNumberLiteral { loc, value, .. } => Ok((*loc, value.clone())),
        Expression::Cast { loc, expr, .. } => Ok((*loc, eval_const_rational(expr, ns)?.1)),
        Expression::Negate { loc, expr, .. } => Ok((*loc, -eval_const_rational(expr, ns)?.1)),
        Expression::ConstantVariable {
            contract_no: Some(contract_no),
            var_no,
            ..
        } => {
            let expr = ns.contracts[*contract_no].variables[*var_no]
                .initializer
                .as_ref()
                .unwrap()
                .clone();

            eval_const_rational(&expr, ns)
        }
        Expression::ConstantVariable {
            contract_no: None,
            var_no,
            ..
        } => {
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
        Expression::Add { .. }
        | Expression::Subtract { .. }
        | Expression::Multiply { .. }
        | Expression::Divide { .. }
        | Expression::Modulo { .. }
        | Expression::Power { .. }
        | Expression::ShiftLeft { .. }
        | Expression::ShiftRight { .. }
        | Expression::BitwiseAnd { .. }
        | Expression::BitwiseOr { .. }
        | Expression::BitwiseXor { .. }
        | Expression::NumberLiteral { .. } => match eval_constants_in_expression(expr, ns) {
            (
                Some(Expression::NumberLiteral {
                    loc,
                    ty,
                    value: result,
                }),
                _,
            ) => {
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
        Expression::Add {
            loc,
            ty,
            unchecked: _,
            left,
            right,
        } => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral { value: left, .. }),
                Some(Expression::NumberLiteral { value: right, .. }),
            ) = (left, right)
            {
                (
                    Some(Expression::NumberLiteral {
                        loc: *loc,
                        ty: ty.clone(),
                        value: left.add(right),
                    }),
                    true,
                )
            } else {
                (None, true)
            }
        }
        Expression::Subtract {
            loc,
            ty,
            unchecked: _,
            left,
            right,
        } => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral { value: left, .. }),
                Some(Expression::NumberLiteral { value: right, .. }),
            ) = (&left, &right)
            {
                (
                    Some(Expression::NumberLiteral {
                        loc: *loc,
                        ty: ty.clone(),
                        value: left.sub(right),
                    }),
                    true,
                )
            } else {
                (None, true)
            }
        }

        Expression::Multiply {
            loc,
            ty,
            unchecked: _,
            left,
            right,
        } => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral { value: left, .. }),
                Some(Expression::NumberLiteral { value: right, .. }),
            ) = (&left, &right)
            {
                (
                    Some(Expression::NumberLiteral {
                        loc: *loc,
                        ty: ty.clone(),
                        value: left.mul(right),
                    }),
                    true,
                )
            } else {
                (None, true)
            }
        }
        Expression::Divide {
            loc,
            ty,
            left,
            right,
        } => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral { value: left, .. }),
                Some(Expression::NumberLiteral { value: right, .. }),
            ) = (&left, &right)
            {
                if right.is_zero() {
                    ns.diagnostics
                        .push(Diagnostic::error(*loc, "divide by zero".to_string()));
                    (None, false)
                } else {
                    (
                        Some(Expression::NumberLiteral {
                            loc: *loc,
                            ty: ty.clone(),
                            value: left.div(right),
                        }),
                        true,
                    )
                }
            } else {
                (None, true)
            }
        }

        Expression::Modulo {
            loc,
            ty,
            left,
            right,
        } => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral { value: left, .. }),
                Some(Expression::NumberLiteral { value: right, .. }),
            ) = (&left, &right)
            {
                if right.is_zero() {
                    ns.diagnostics
                        .push(Diagnostic::error(*loc, "divide by zero".to_string()));
                    (None, false)
                } else {
                    (
                        Some(Expression::NumberLiteral {
                            loc: *loc,
                            ty: ty.clone(),
                            value: left % right,
                        }),
                        true,
                    )
                }
            } else {
                (None, true)
            }
        }
        Expression::Power {
            loc,
            ty,
            unchecked: _,
            base,
            exp,
        } => {
            let base = eval_constants_in_expression(base, ns).0;
            let exp = eval_constants_in_expression(exp, ns).0;

            if let (
                Some(Expression::NumberLiteral { value: left, .. }),
                Some(Expression::NumberLiteral {
                    loc: right_loc,
                    value: right,
                    ..
                }),
            ) = (&base, &exp)
            {
                if overflow_check(right, &Type::Uint(16), right_loc).is_some() {
                    ns.diagnostics.push(Diagnostic::error(
                        *right_loc,
                        format!("power by {right} is not possible"),
                    ));
                    (None, false)
                } else {
                    (
                        Some(Expression::NumberLiteral {
                            loc: *loc,
                            ty: ty.clone(),
                            value: left.pow(right.to_u16().unwrap().into()),
                        }),
                        true,
                    )
                }
            } else {
                (None, true)
            }
        }
        Expression::ShiftLeft {
            loc,
            ty,
            left,
            right,
        } => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral { value: left, .. }),
                Some(Expression::NumberLiteral {
                    loc: right_loc,
                    value: right,
                    ..
                }),
            ) = (&left, &right)
            {
                if overflow_check(right, &Type::Uint(64), right_loc).is_some() {
                    ns.diagnostics.push(Diagnostic::error(
                        *right_loc,
                        format!("left shift by {right} is not possible"),
                    ));
                    (None, false)
                } else {
                    if right >= &BigInt::from(left.bits()) {
                        ns.diagnostics.push(Diagnostic::warning(
                            *right_loc,
                            format!("left shift by {right} may overflow the final result"),
                        ));
                    }

                    (
                        Some(Expression::NumberLiteral {
                            loc: *loc,
                            ty: ty.clone(),
                            value: left.shl(right.to_u64().unwrap()),
                        }),
                        true,
                    )
                }
            } else {
                (None, true)
            }
        }

        Expression::ShiftRight {
            loc,
            ty,
            left,
            right,
            sign: _,
        } => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral { value: left, .. }),
                Some(Expression::NumberLiteral {
                    loc: right_loc,
                    value: right,
                    ..
                }),
            ) = (&left, &right)
            {
                if overflow_check(right, &Type::Uint(64), right_loc).is_some() {
                    ns.diagnostics.push(Diagnostic::error(
                        *right_loc,
                        format!("right shift by {right} is not possible"),
                    ));
                    (None, false)
                } else {
                    (
                        Some(Expression::NumberLiteral {
                            loc: *loc,
                            ty: ty.clone(),
                            value: left.shr(right.to_u64().unwrap()),
                        }),
                        true,
                    )
                }
            } else {
                (None, true)
            }
        }
        Expression::BitwiseAnd {
            loc,
            ty,
            left,
            right,
        } => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral { value: left, .. }),
                Some(Expression::NumberLiteral { value: right, .. }),
            ) = (&left, &right)
            {
                (
                    Some(Expression::NumberLiteral {
                        loc: *loc,
                        ty: ty.clone(),
                        value: left.bitand(right),
                    }),
                    true,
                )
            } else {
                (None, true)
            }
        }
        Expression::BitwiseOr {
            loc,
            ty,
            left,
            right,
        } => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral { value: left, .. }),
                Some(Expression::NumberLiteral { value: right, .. }),
            ) = (&left, &right)
            {
                (
                    Some(Expression::NumberLiteral {
                        loc: *loc,
                        ty: ty.clone(),
                        value: left.bitor(right),
                    }),
                    true,
                )
            } else {
                (None, true)
            }
        }
        Expression::BitwiseXor {
            loc,
            ty,
            left,
            right,
        } => {
            let left = eval_constants_in_expression(left, ns).0;
            let right = eval_constants_in_expression(right, ns).0;

            if let (
                Some(Expression::NumberLiteral { value: left, .. }),
                Some(Expression::NumberLiteral { value: right, .. }),
            ) = (&left, &right)
            {
                (
                    Some(Expression::NumberLiteral {
                        loc: *loc,
                        ty: ty.clone(),
                        value: left.bitxor(right),
                    }),
                    true,
                )
            } else {
                (None, true)
            }
        }
        Expression::ZeroExt { loc, to, expr } => {
            let expr = eval_constants_in_expression(expr, ns).0;
            if let Some(Expression::NumberLiteral { value, .. }) = expr {
                (
                    Some(Expression::NumberLiteral {
                        loc: *loc,
                        ty: to.clone(),
                        value,
                    }),
                    true,
                )
            } else {
                (None, true)
            }
        }
        Expression::SignExt { loc, to, expr } => {
            let expr = eval_constants_in_expression(expr, ns).0;
            if let Some(Expression::NumberLiteral { value, .. }) = expr {
                (
                    Some(Expression::NumberLiteral {
                        loc: *loc,
                        ty: to.clone(),
                        value,
                    }),
                    true,
                )
            } else {
                (None, true)
            }
        }
        Expression::NumberLiteral { .. } => (Some(expr.clone()), true),
        _ => (None, true),
    }
}

/// Function that takes a BigInt and an expected type. If the number of bits in the type required to represent the BigInt is not suffiecient, it will return a diagnostic.
pub(super) fn overflow_check(result: &BigInt, ty: &Type, loc: &Loc) -> Option<Diagnostic> {
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
