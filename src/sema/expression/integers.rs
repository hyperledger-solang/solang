// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{Expression, Namespace, Type};
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::ResolveTo;
use num_bigint::{BigInt, Sign};
use num_traits::Zero;
use solang_parser::diagnostics::Diagnostic;
use solang_parser::pt;

pub(super) fn coerce(
    l: &Type,
    l_loc: &pt::Loc,
    r: &Type,
    r_loc: &pt::Loc,
    ns: &Namespace,
    diagnostics: &mut Diagnostics,
) -> Result<Type, ()> {
    let l = match l {
        Type::Ref(ty) => ty,
        Type::StorageRef(_, ty) => ty,
        _ => l,
    };
    let r = match r {
        Type::Ref(ty) => ty,
        Type::StorageRef(_, ty) => ty,
        _ => r,
    };

    if *l == *r {
        return Ok(l.clone());
    }

    // Address payable is implicitly convertible to address, so we can compare these
    if *l == Type::Address(false) && *r == Type::Address(true)
        || *l == Type::Address(true) && *r == Type::Address(false)
    {
        return Ok(Type::Address(false));
    }

    coerce_number(l, l_loc, r, r_loc, true, false, ns, diagnostics)
}

/// Calculate the number of bits and the sign of a type, or generate a diagnostic
/// that the type that is not allowed.
pub(super) fn type_bits_and_sign(
    l: &Type,
    l_loc: &pt::Loc,
    allow_bytes: bool,
    ns: &Namespace,
    diagnostics: &mut Diagnostics,
) -> Result<(u16, bool), ()> {
    match l {
        Type::Uint(n) => Ok((*n, false)),
        Type::Int(n) => Ok((*n, true)),
        Type::Value => Ok((ns.value_length as u16 * 8, false)),
        Type::FunctionSelector => Ok((ns.target.selector_length() as u16 * 8, false)),
        Type::Bytes(n) if allow_bytes => Ok((*n as u16 * 8, false)),
        Type::Enum(n) => {
            diagnostics.push(Diagnostic::error(
                *l_loc,
                format!("type enum {} not allowed", ns.enums[*n]),
            ));
            Err(())
        }
        Type::Struct(str_ty) => {
            diagnostics.push(Diagnostic::error(
                *l_loc,
                format!("type struct {} not allowed", str_ty.definition(ns)),
            ));
            Err(())
        }
        Type::Array(..) => {
            diagnostics.push(Diagnostic::error(
                *l_loc,
                format!("type array {} not allowed", l.to_string(ns)),
            ));
            Err(())
        }
        Type::Ref(n) => type_bits_and_sign(n, l_loc, allow_bytes, ns, diagnostics),
        Type::StorageRef(_, n) => type_bits_and_sign(n, l_loc, allow_bytes, ns, diagnostics),
        _ => {
            diagnostics.push(Diagnostic::error(
                *l_loc,
                format!("expression of type {} not allowed", l.to_string(ns)),
            ));
            Err(())
        }
    }
}

pub fn coerce_number(
    l: &Type,
    l_loc: &pt::Loc,
    r: &Type,
    r_loc: &pt::Loc,
    allow_bytes: bool,
    for_compare: bool,
    ns: &Namespace,
    diagnostics: &mut Diagnostics,
) -> Result<Type, ()> {
    let l = match l {
        Type::Ref(ty) => ty,
        Type::StorageRef(_, ty) => ty,
        _ => l,
    };
    let r = match r {
        Type::Ref(ty) => ty,
        Type::StorageRef(_, ty) => ty,
        _ => r,
    };

    match (l, r) {
        (Type::Address(false), Type::Address(false)) if for_compare => {
            return Ok(Type::Address(false));
        }
        (Type::Address(true), Type::Address(true)) if for_compare => {
            return Ok(Type::Address(true));
        }
        (Type::Contract(left), Type::Contract(right)) if left == right && for_compare => {
            return Ok(Type::Contract(*left));
        }
        (Type::Bytes(left_length), Type::Bytes(right_length)) if allow_bytes => {
            return Ok(Type::Bytes(std::cmp::max(*left_length, *right_length)));
        }
        (Type::Bytes(_), _) if allow_bytes => {
            return Ok(l.clone());
        }
        (_, Type::Bytes(_)) if allow_bytes => {
            return Ok(r.clone());
        }
        (Type::FunctionSelector, _) | (_, Type::FunctionSelector) if allow_bytes => {
            return Ok(Type::Bytes(ns.target.selector_length()));
        }
        (Type::Rational, Type::Int(_)) => {
            return Ok(Type::Rational);
        }
        (Type::Rational, Type::Rational) => {
            return Ok(Type::Rational);
        }
        (Type::Rational, Type::Uint(_)) => {
            return Ok(Type::Rational);
        }
        (Type::Uint(_), Type::Rational) => {
            return Ok(Type::Rational);
        }
        (Type::Int(_), Type::Rational) => {
            return Ok(Type::Rational);
        }
        (Type::Bool, Type::Int(_) | Type::Uint(_)) => {
            return Ok(r.clone());
        }
        (Type::Int(_) | Type::Uint(_), Type::Bool) => {
            return Ok(l.clone());
        }
        _ => (),
    }

    let (left_len, left_signed) = type_bits_and_sign(l, l_loc, false, ns, diagnostics)?;

    let (right_len, right_signed) = type_bits_and_sign(r, r_loc, false, ns, diagnostics)?;

    Ok(match (left_signed, right_signed) {
        (true, true) => Type::Int(left_len.max(right_len)),
        (false, false) => Type::Uint(left_len.max(right_len)),
        (true, false) => {
            // uint8 fits into int16
            let len = left_len.max(right_len + 8);

            Type::Int(len.min(256))
        }
        (false, true) => {
            // uint8 fits into int16
            let len = (left_len + 8).max(right_len);

            Type::Int(len.min(256))
        }
    })
}

/// Try to convert a BigInt into a Expression::NumberLiteral.
/// The `hex_str_len` parameter is used to specify a custom length for 0-prefixed hex-literals.
pub fn bigint_to_expression(
    loc: &pt::Loc,
    n: &BigInt,
    ns: &Namespace,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
    hex_str_len: Option<usize>,
) -> Result<Expression, ()> {
    if let ResolveTo::Type(resolve_to) = resolve_to {
        if *resolve_to != Type::Unresolved {
            if !(resolve_to.is_integer(ns) || matches!(resolve_to, Type::Bytes(_)) && n.is_zero()) {
                diagnostics.push(Diagnostic::cast_error(
                    *loc,
                    format!("expected '{}', found integer", resolve_to.to_string(ns)),
                ));
                return Err(());
            }

            return Ok(Expression::NumberLiteral {
                loc: *loc,
                ty: resolve_to.clone(),
                value: n.clone(),
            });
        }
    }

    // BigInt bits() returns the bits used without the sign. Negative value is allowed to be one
    // larger than positive value, e.g int8 has inclusive range -128 to 127.
    let bits = if n.sign() == Sign::Minus {
        (n + 1u32).bits()
    } else {
        n.bits()
    };

    // Return smallest type; hex literals with odd length are not allowed.
    let int_size = hex_str_len
        .map(|v| if v % 2 == 0 { v as u64 * 4 } else { bits })
        .unwrap_or_else(|| if bits < 7 { 8 } else { (bits + 7) & !7 }) as u16;

    if n.sign() == Sign::Minus {
        if bits > 255 {
            diagnostics.push(Diagnostic::error(*loc, format!("{n} is too large")));
            Err(())
        } else {
            Ok(Expression::NumberLiteral {
                loc: *loc,
                ty: Type::Int(int_size),
                value: n.clone(),
            })
        }
    } else if bits > 256 {
        diagnostics.push(Diagnostic::error(*loc, format!("{n} is too large")));
        Err(())
    } else {
        Ok(Expression::NumberLiteral {
            loc: *loc,
            ty: Type::Uint(int_size),
            value: n.clone(),
        })
    }
}
