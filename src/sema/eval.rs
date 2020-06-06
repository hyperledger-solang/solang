use num_bigint::BigInt;
use num_bigint::Sign;
use num_traits::One;
use num_traits::ToPrimitive;
use num_traits::Zero;

use super::ast::{Expression, Namespace};
use output::Output;
use parser::pt::Loc;

/// Resolve an expression where a compile-time constant is expected
pub fn eval_const_number(
    expr: &Expression,
    contract_no: Option<usize>,
    ns: &Namespace,
) -> Result<(Loc, BigInt), Output> {
    match expr {
        Expression::Add(loc, _, l, r) => Ok((
            *loc,
            eval_const_number(l, contract_no, ns)?.1 + eval_const_number(r, contract_no, ns)?.1,
        )),
        Expression::Subtract(loc, _, l, r) => Ok((
            *loc,
            eval_const_number(l, contract_no, ns)?.1 - eval_const_number(r, contract_no, ns)?.1,
        )),
        Expression::Multiply(loc, _, l, r) => Ok((
            *loc,
            eval_const_number(l, contract_no, ns)?.1 * eval_const_number(r, contract_no, ns)?.1,
        )),
        Expression::UDivide(loc, _, l, r) | Expression::SDivide(loc, _, l, r) => {
            let divisor = eval_const_number(r, contract_no, ns)?.1;

            if divisor.is_zero() {
                Err(Output::error(*loc, "divide by zero".to_string()))
            } else {
                Ok((*loc, eval_const_number(l, contract_no, ns)?.1 / divisor))
            }
        }
        Expression::UModulo(loc, _, l, r) | Expression::SModulo(loc, _, l, r) => {
            let divisor = eval_const_number(r, contract_no, ns)?.1;

            if divisor.is_zero() {
                Err(Output::error(*loc, "divide by zero".to_string()))
            } else {
                Ok((*loc, eval_const_number(l, contract_no, ns)?.1 % divisor))
            }
        }
        Expression::BitwiseAnd(loc, _, l, r) => Ok((
            *loc,
            eval_const_number(l, contract_no, ns)?.1 & eval_const_number(r, contract_no, ns)?.1,
        )),
        Expression::BitwiseOr(loc, _, l, r) => Ok((
            *loc,
            eval_const_number(l, contract_no, ns)?.1 | eval_const_number(r, contract_no, ns)?.1,
        )),
        Expression::BitwiseXor(loc, _, l, r) => Ok((
            *loc,
            eval_const_number(l, contract_no, ns)?.1 ^ eval_const_number(r, contract_no, ns)?.1,
        )),
        Expression::Power(loc, _, base, exp) => {
            let b = eval_const_number(base, contract_no, ns)?.1;
            let mut e = eval_const_number(exp, contract_no, ns)?.1;

            if e.sign() == Sign::Minus {
                Err(Output::error(
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
            let l = eval_const_number(left, contract_no, ns)?.1;
            let r = eval_const_number(right, contract_no, ns)?.1;
            let r = match r.to_usize() {
                Some(r) => r,
                None => {
                    return Err(Output::error(
                        expr.loc(),
                        format!("cannot left shift by {}", r),
                    ));
                }
            };
            Ok((*loc, l << r))
        }
        Expression::ShiftRight(loc, _, left, right, _) => {
            let l = eval_const_number(left, contract_no, ns)?.1;
            let r = eval_const_number(right, contract_no, ns)?.1;
            let r = match r.to_usize() {
                Some(r) => r,
                None => {
                    return Err(Output::error(
                        expr.loc(),
                        format!("cannot right shift by {}", r),
                    ));
                }
            };
            Ok((*loc, l >> r))
        }
        Expression::NumberLiteral(loc, _, n) => Ok((*loc, n.clone())),
        Expression::ZeroExt(loc, _, n) => Ok((*loc, eval_const_number(n, contract_no, ns)?.1)),
        Expression::Not(loc, n) => Ok((*loc, !eval_const_number(n, contract_no, ns)?.1)),
        Expression::Complement(loc, _, n) => Ok((*loc, !eval_const_number(n, contract_no, ns)?.1)),
        Expression::UnaryMinus(loc, _, n) => Ok((*loc, -eval_const_number(n, contract_no, ns)?.1)),
        Expression::ConstantVariable(_, _, no) => {
            let expr = ns.contracts[contract_no.unwrap()].variables[*no]
                .initializer
                .as_ref()
                .unwrap()
                .clone();

            eval_const_number(&expr, contract_no, ns)
        }
        _ => Err(Output::error(
            expr.loc(),
            "expression not allowed in constant number expression".to_string(),
        )),
    }
}
