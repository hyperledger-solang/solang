// SPDX-License-Identifier: Apache-2.0

use num_bigint::BigInt;
use num_bigint::Sign;
use num_rational::BigRational;
use num_traits::One;
use num_traits::ToPrimitive;
use num_traits::Zero;

use super::ast::{Diagnostic, Expression, Namespace};
use solang_parser::pt;
use solang_parser::pt::CodeLocation;
use std::ops::Shl;
use std::ops::{Add, Mul, Sub};

use crate::sema::ast::Type;
use solang_parser::pt::Loc;

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

pub fn eval_constants_in_expression(expr: &Expression, ns: &mut Namespace) -> Expression {
    match expr {
        Expression::Add(loc, ty, unchecked, left, right) => {
            let left = eval_constants_in_expression(left, ns);
            let right = eval_constants_in_expression(right, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left, &right)
            {
                overflow_check(ns, left.add(right), ty.clone(), *loc);
                bigint_to_expression(loc, ty, left.add(right))
            } else {
                Expression::Add(
                    *loc,
                    ty.clone(),
                    *unchecked,
                    Box::new(left),
                    Box::new(right),
                )
            }
        }
        Expression::Subtract(loc, ty, unchecked, left, right) => {
            let left = eval_constants_in_expression(left, ns);
            let right = eval_constants_in_expression(right, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left, &right)
            {
                overflow_check(ns, left.sub(right), ty.clone(), *loc);
                bigint_to_expression(loc, ty, left.sub(right))
            } else {
                Expression::Subtract(
                    *loc,
                    ty.clone(),
                    *unchecked,
                    Box::new(left),
                    Box::new(right),
                )
            }
        }

        Expression::Multiply(loc, ty, unchecked, left, right) => {
            let left = eval_constants_in_expression(left, ns);
            let right = eval_constants_in_expression(right, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left, &right)
            {
                overflow_check(ns, left.mul(right.to_u32().unwrap()), ty.clone(), *loc);
                bigint_to_expression(loc, ty, left.mul(right.to_u32().unwrap()))
            } else {
                Expression::Multiply(
                    *loc,
                    ty.clone(),
                    *unchecked,
                    Box::new(left),
                    Box::new(right),
                )
            }
        }

        Expression::Power(loc, ty, unchecked, left, right) => {
            let left = eval_constants_in_expression(left, ns);
            let right = eval_constants_in_expression(right, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left, &right)
            {
                overflow_check(ns, left.pow(right.to_u32().unwrap()), ty.clone(), *loc);
                bigint_to_expression(loc, ty, left.pow(right.to_u32().unwrap()))
            } else {
                Expression::Power(
                    *loc,
                    ty.clone(),
                    *unchecked,
                    Box::new(left),
                    Box::new(right),
                )
            }
        }

        Expression::ShiftLeft(loc, ty, left, right) => {
            let left = eval_constants_in_expression(left, ns);
            let right = eval_constants_in_expression(right, ns);

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left, &right)
            {
                overflow_check(ns, left.shl(right.to_u32().unwrap()), ty.clone(), *loc);
                bigint_to_expression(loc, ty, left.shl(right.to_u32().unwrap()))
            } else {
                Expression::ShiftLeft(*loc, ty.clone(), Box::new(left), Box::new(right))
            }
        }
        _ => expr.clone(),
    }
}

fn bigint_to_expression(loc: &Loc, ty: &Type, n: BigInt) -> Expression {
    let n = match ty {
        Type::Uint(bits) => {
            if n.bits() > *bits as u64 {
                let (_, mut bs) = n.to_bytes_le();
                bs.truncate(*bits as usize / 8);

                BigInt::from_bytes_le(Sign::Plus, &bs)
            } else {
                n
            }
        }
        Type::Int(bits) => {
            if n.bits() > *bits as u64 {
                let mut bs = n.to_signed_bytes_le();
                bs.truncate(*bits as usize / 8);

                BigInt::from_signed_bytes_le(&bs)
            } else {
                n
            }
        }
        Type::StorageRef(..) => n,
        _ => unreachable!(),
    };

    Expression::NumberLiteral(*loc, ty.clone(), n)
}

fn overflow_check(ns: &mut Namespace, result: BigInt, ty: Type, loc: Loc) {
    if let Type::Uint(bits) = ty {
        // If the result sign is minus, throw an error.
        if let Sign::Minus = result.sign() {
            ns.diagnostics.push(Diagnostic::error(
                loc,
            format!( "Type int_const {:?} is not implicitly convertible to expected type {:?}. Cannot implicitly convert signed literal to unsigned type.",result,ty),
            ));
        }

        // If bits of the result is more than bits of the type, throw and error.
        if result.bits() > bits as u64 {
            ns.diagnostics.push(Diagnostic::error(
                loc,
                format!("Type int_const {:?} is not implicitly convertible to expected type {:?}. Literal is too large to fit in {:?}.",result,ty,ty),
            ));
        }
    }

    if let Type::Int(bits) = ty {
        // If number of bits is more than what the type can hold. BigInt.bits() is not used here since it disregards the sign.
        if result.to_signed_bytes_be().len() * 8 > (bits as usize) {
            ns.diagnostics.push(Diagnostic::error(
                loc,
                format!("Type int_const {:?} is not implicitly convertible to expected type {:?}. Literal is too large to fit in {:?}.",result,ty,ty),
            ));
        }
    }
}
