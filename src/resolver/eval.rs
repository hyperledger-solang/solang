use num_bigint::BigInt;
use num_bigint::Sign;
use num_traits::One;
use num_traits::ToPrimitive;
use num_traits::Zero;

use super::expression::Expression;
use output;
use output::Output;
use parser::pt::Loc;

/// Resolve an expression where a compile-time constant is expected
pub fn eval_number_expression(
    expr: &Expression,
    errors: &mut Vec<output::Output>,
) -> Result<(Loc, BigInt), ()> {
    match expr {
        Expression::Add(loc, l, r) => Ok((
            *loc,
            eval_number_expression(l, errors)?.1 + eval_number_expression(r, errors)?.1,
        )),
        Expression::Subtract(loc, l, r) => Ok((
            *loc,
            eval_number_expression(l, errors)?.1 - eval_number_expression(r, errors)?.1,
        )),
        Expression::Multiply(loc, l, r) => Ok((
            *loc,
            eval_number_expression(l, errors)?.1 * eval_number_expression(r, errors)?.1,
        )),
        Expression::UDivide(loc, l, r) | Expression::SDivide(loc, l, r) => {
            let divisor = eval_number_expression(r, errors)?.1;

            if divisor.is_zero() {
                errors.push(Output::error(*loc, "divide by zero".to_string()));

                Err(())
            } else {
                Ok((*loc, eval_number_expression(l, errors)?.1 / divisor))
            }
        }
        Expression::UModulo(loc, l, r) | Expression::SModulo(loc, l, r) => {
            let divisor = eval_number_expression(r, errors)?.1;

            if divisor.is_zero() {
                errors.push(Output::error(*loc, "divide by zero".to_string()));

                Err(())
            } else {
                Ok((*loc, eval_number_expression(l, errors)?.1 % divisor))
            }
        }
        Expression::BitwiseAnd(loc, l, r) => Ok((
            *loc,
            eval_number_expression(l, errors)?.1 & eval_number_expression(r, errors)?.1,
        )),
        Expression::BitwiseOr(loc, l, r) => Ok((
            *loc,
            eval_number_expression(l, errors)?.1 | eval_number_expression(r, errors)?.1,
        )),
        Expression::BitwiseXor(loc, l, r) => Ok((
            *loc,
            eval_number_expression(l, errors)?.1 ^ eval_number_expression(r, errors)?.1,
        )),
        Expression::Power(loc, base, exp) => {
            let b = eval_number_expression(base, errors)?.1;
            let mut e = eval_number_expression(exp, errors)?.1;

            if e.sign() == Sign::Minus {
                errors.push(Output::error(
                    expr.loc(),
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
        Expression::ShiftLeft(loc, left, right) => {
            let l = eval_number_expression(left, errors)?.1;
            let r = eval_number_expression(right, errors)?.1;
            let r = match r.to_usize() {
                Some(r) => r,
                None => {
                    errors.push(Output::error(
                        expr.loc(),
                        format!("cannot left shift by {}", r),
                    ));

                    return Err(());
                }
            };
            Ok((*loc, l << r))
        }
        Expression::ShiftRight(loc, left, right, _) => {
            let l = eval_number_expression(left, errors)?.1;
            let r = eval_number_expression(right, errors)?.1;
            let r = match r.to_usize() {
                Some(r) => r,
                None => {
                    errors.push(Output::error(
                        expr.loc(),
                        format!("cannot right shift by {}", r),
                    ));

                    return Err(());
                }
            };
            Ok((*loc, l >> r))
        }
        Expression::NumberLiteral(loc, _, n) => Ok((*loc, n.clone())),
        Expression::ZeroExt(loc, _, n) => Ok((*loc, eval_number_expression(n, errors)?.1)),
        Expression::Not(loc, n) => Ok((*loc, !eval_number_expression(n, errors)?.1)),
        Expression::Complement(loc, n) => Ok((*loc, !eval_number_expression(n, errors)?.1)),
        Expression::UnaryMinus(loc, n) => Ok((*loc, -eval_number_expression(n, errors)?.1)),
        _ => {
            errors.push(Output::error(
                expr.loc(),
                "expression not allowed in constant number expression".to_string(),
            ));

            Err(())
        }
    }
}
