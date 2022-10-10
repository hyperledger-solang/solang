// SPDX-License-Identifier: Apache-2.0

use super::ast::{Diagnostic, Expression, Namespace, Type};
use super::Recurse;
use num_bigint::BigInt;
use num_bigint::Sign;
use num_rational::BigRational;
use num_traits::One;
use num_traits::ToPrimitive;
use num_traits::Zero;
use solang_parser::pt;
use solang_parser::pt::{CodeLocation, Loc};
use std::ops::{Add, Div, Mul, Shl, Shr, Sub};

struct RecurseParams<'a> {
    recursed: &'a mut Vec<Expression>,
    diagnostics: &'a mut Vec<Diagnostic>,
}

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

//// Helper function that recurses the expression, and calls expression = eval_constants_in_expression().
/// If expression is an arithmetic operation of two number literals, it will be stored in an expression list to later be evaluated by verify_expression_for_overflow().
/// If the return is a diagnostic, store it in a diagnostics list.
fn check_term_for_constant_overflow(expr: &Expression, params: &mut RecurseParams) -> bool {
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
        | Expression::BitwiseXor(..) => {
            let expression = eval_constants_in_expression(expr.clone());
            match expression {
                Ok(input) => {
                    if let Expression::NumberLiteral(..) = input {
                        params.recursed.push(input);
                        return false;
                    }
                }
                Err(input) => {
                    params.diagnostics.push(input);
                }
            }
        }
        _ => {
            if let Expression::NumberLiteral(..) = expr {
                params.recursed.push(expr.clone());
                return false;
            }
        }
    }

    true
}

/// This function recursively folds number literals in a given expression.
fn eval_constants_in_expression(expr: Expression) -> Result<Expression, Diagnostic> {
    match expr {
        Expression::Add(loc, ty, unchecked, left, right) => {
            let left = eval_constants_in_expression(*left)?;
            let right = eval_constants_in_expression(*right)?;

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left, &right)
            {
                Ok(Expression::NumberLiteral(loc, ty, left.add(right)))
            } else {
                Ok(Expression::Add(
                    loc,
                    ty,
                    unchecked,
                    Box::new(left),
                    Box::new(right),
                ))
            }
        }
        Expression::Subtract(loc, ty, unchecked, left, right) => {
            let left = eval_constants_in_expression(*left)?;
            let right = eval_constants_in_expression(*right)?;

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left, &right)
            {
                Ok(Expression::NumberLiteral(loc, ty, left.sub(right)))
            } else {
                Ok(Expression::Subtract(
                    loc,
                    ty,
                    unchecked,
                    Box::new(left),
                    Box::new(right),
                ))
            }
        }

        Expression::Multiply(loc, ty, unchecked, left, right) => {
            let left = eval_constants_in_expression(*left)?;
            let right = eval_constants_in_expression(*right)?;

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left, &right)
            {
                Ok(Expression::NumberLiteral(loc, ty, left.mul(right)))
            } else {
                Ok(Expression::Multiply(
                    loc,
                    ty,
                    unchecked,
                    Box::new(left),
                    Box::new(right),
                ))
            }
        }
        Expression::Divide(loc, ty, left, right) => {
            let left = eval_constants_in_expression(*left)?;
            let right = eval_constants_in_expression(*right)?;

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left, &right)
            {
                if right.is_zero() {
                    Err(Diagnostic::error(loc, "divide by zero".to_string()))
                } else {
                    Ok(Expression::NumberLiteral(loc, ty, left.div(right)))
                }
            } else {
                Ok(Expression::Divide(loc, ty, Box::new(left), Box::new(right)))
            }
        }

        Expression::Modulo(loc, ty, left, right) => {
            let left = eval_constants_in_expression(*left)?;
            let right = eval_constants_in_expression(*right)?;

            if let (Expression::NumberLiteral(_, _, left), Expression::NumberLiteral(_, _, right)) =
                (&left, &right)
            {
                if right.is_zero() {
                    Err(Diagnostic::error(loc, "divide by zero".to_string()))
                } else {
                    Ok(Expression::NumberLiteral(loc, ty, left % right))
                }
            } else {
                Ok(Expression::Modulo(loc, ty, Box::new(left), Box::new(right)))
            }
        }
        Expression::Power(loc, ty, unchecked, left, right) => {
            let left = eval_constants_in_expression(*left)?;
            let right = eval_constants_in_expression(*right)?;

            if let (
                Expression::NumberLiteral(_, _, left),
                Expression::NumberLiteral(right_loc, _, right),
            ) = (&left, &right)
            {
                if let Some(diagnostic) = overflow_check(right.clone(), Type::Uint(16), *right_loc)
                {
                    Err(diagnostic)
                } else {
                    Ok(Expression::NumberLiteral(
                        loc,
                        ty,
                        left.pow(right.to_u16().unwrap().into()),
                    ))
                }
            } else {
                Ok(Expression::Power(
                    loc,
                    ty,
                    unchecked,
                    Box::new(left),
                    Box::new(right),
                ))
            }
        }
        Expression::ShiftLeft(loc, ty, left, right) => {
            let left = eval_constants_in_expression(*left)?;
            let right = eval_constants_in_expression(*right)?;

            if let (
                Expression::NumberLiteral(_, _, left),
                Expression::NumberLiteral(right_loc, _, right),
            ) = (&left, &right)
            {
                if let Some(diagnostic) = overflow_check(right.clone(), Type::Uint(64), *right_loc)
                {
                    Err(diagnostic)
                } else {
                    Ok(Expression::NumberLiteral(
                        loc,
                        ty,
                        left.shl(right.to_u64().unwrap()),
                    ))
                }
            } else {
                Ok(Expression::ShiftLeft(
                    loc,
                    ty,
                    Box::new(left),
                    Box::new(right),
                ))
            }
        }

        Expression::ShiftRight(loc, ty, left, right, bool) => {
            let left = eval_constants_in_expression(*left)?;
            let right = eval_constants_in_expression(*right)?;

            if let (
                Expression::NumberLiteral(_, _, left),
                Expression::NumberLiteral(right_loc, _, right),
            ) = (&left, &right)
            {
                if let Some(diagnostic) = overflow_check(right.clone(), Type::Uint(64), *right_loc)
                {
                    Err(diagnostic)
                } else {
                    Ok(Expression::NumberLiteral(
                        loc,
                        ty,
                        left.shr(right.to_u64().unwrap()),
                    ))
                }
            } else {
                Ok(Expression::ShiftRight(
                    loc,
                    ty,
                    Box::new(left),
                    Box::new(right),
                    bool,
                ))
            }
        }
        Expression::Builtin(.., ref args) => {
            for args_iter in args {
                let expression = eval_constants_in_expression(args_iter.clone())?;
                if let Expression::NumberLiteral(loc, ty, res) = expression {
                    if let Some(diagnostic) = overflow_check(res, ty, loc) {
                        return Err(diagnostic);
                    }
                }
            }

            Ok(expr)
        }
        Expression::ZeroExt(loc, ty, expr) => {
            let expr = eval_constants_in_expression(*expr)?;
            if let Expression::NumberLiteral(_, _, n) = expr {
                Ok(Expression::NumberLiteral(loc, ty, n))
            } else {
                Ok(Expression::ZeroExt(loc, ty, Box::new(expr)))
            }
        }
        Expression::SignExt(loc, ty, expr) => {
            let expr = eval_constants_in_expression(*expr)?;
            if let Expression::NumberLiteral(_, _, n) = expr {
                Ok(Expression::NumberLiteral(loc, ty, n))
            } else {
                Ok(Expression::SignExt(loc, ty, Box::new(expr)))
            }
        }

        _ => Ok(expr),
    }
}

/// Function that takes a BigInt and an expected type. If the number of bits in the type required to represent the BigInt is not suffiecient, it will return a diagnostic.
fn overflow_check(result: BigInt, ty: Type, loc: Loc) -> Option<Diagnostic> {
    println!("result {}", result.bits());
    if let Type::Uint(bits) = ty {
        // If the result sign is minus, throw an error.
        if let Sign::Minus = result.sign() {
            return Some(Diagnostic::error(
                loc,
            format!( "negative value {} does not fit into type uint{}. Cannot implicitly convert signed literal to unsigned type.",result,ty.get_type_size()),
            ));
        }

        // If bits of the result is more than bits of the type, throw and error.
        if result.bits() > bits as u64 {
            return Some(Diagnostic::error(
                loc,
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
        if result.to_signed_bytes_be().len() * 8 > (bits as usize) {
            return Some(Diagnostic::error(
                loc,
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

/// Wrapper function for the overflow detection logic.
/// It first calls a function that recurses a given expression and returns a list of arithmetic operations within the expression, and a list of diagnostics.
/// Then, it calls overflow_check() on the retrieved arithmetic operations, and pushes the returned diagnostics to the namespace
pub(super) fn verify_expression_for_overflow(expr: Expression, ns: &mut Namespace) {
    let recursed: &mut Vec<Expression> = &mut Vec::new();
    let diagnostics: &mut Vec<Diagnostic> = &mut Vec::new();
    let mut params = RecurseParams {
        recursed,
        diagnostics,
    };

    expr.recurse(&mut params, check_term_for_constant_overflow);

    for iter in params.recursed {
        if let Expression::NumberLiteral(loc, ty, res) = iter.clone() {
            if let Some(diagnostic) = overflow_check(res, ty, loc) {
                ns.diagnostics.push(diagnostic);
            }
        }
    }
    ns.diagnostics.append(params.diagnostics);

    match eval_constants_in_expression(expr.clone()) {
        Ok(expression) => {
            if let Expression::NumberLiteral(loc, ty, res) = expression {
                let result = overflow_check(res, ty, loc);
                if let Some(diagnostic) = result {
                    ns.diagnostics.push(diagnostic);
                }
            }
        }
        Err(diagnostic) => {
            ns.diagnostics.push(diagnostic);
        }
    }
}
