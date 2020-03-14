use num_bigint::BigInt;
use num_traits::FromPrimitive;
use num_traits::One;
use num_traits::Zero;

use super::expression::Expression;
use parser::ast;
use resolver;

/// Given a storage slot which is the start of the array, calculate the
/// offset of the array element. This function exists to avoid doing
/// 256 bit multiply if possible.
pub fn array_offset(
    loc: &ast::Loc,
    start: Expression,
    index: Expression,
    elem_ty: resolver::Type,
    ns: &resolver::Contract,
) -> Expression {
    let elem_size = elem_ty.storage_slots(ns);

    // the index needs to be cast to i256 and multiplied by the number
    // of slots for each element
    if elem_size == BigInt::one() {
        Expression::Add(*loc, Box::new(start), Box::new(index))
    } else if (elem_size.clone() & (elem_size.clone() - BigInt::one())) == BigInt::zero() {
        // elem_size is power of 2
        Expression::ShiftLeft(
            *loc,
            Box::new(start),
            Box::new(Expression::ShiftLeft(
                *loc,
                Box::new(index),
                Box::new(Expression::NumberLiteral(
                    *loc,
                    256,
                    BigInt::from_usize(elem_size.bits()).unwrap(),
                )),
            )),
        )
    } else {
        Expression::Add(
            *loc,
            Box::new(start),
            Box::new(Expression::Multiply(
                *loc,
                Box::new(index),
                Box::new(Expression::NumberLiteral(*loc, 256, elem_size)),
            )),
        )
    }
}
