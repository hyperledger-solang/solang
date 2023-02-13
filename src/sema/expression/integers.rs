// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{Expression, Namespace, Type};
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::ResolveTo;
use num_bigint::{BigInt, Sign};
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

pub(super) fn get_int_length(
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
        Type::Ref(n) => get_int_length(n, l_loc, allow_bytes, ns, diagnostics),
        Type::StorageRef(_, n) => get_int_length(n, l_loc, allow_bytes, ns, diagnostics),
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

    let (left_len, left_signed) = get_int_length(l, l_loc, false, ns, diagnostics)?;

    let (right_len, right_signed) = get_int_length(r, r_loc, false, ns, diagnostics)?;

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
    let bits = n.bits();

    if let ResolveTo::Type(resolve_to) = resolve_to {
        if *resolve_to != Type::Unresolved {
            if !resolve_to.is_integer() {
                diagnostics.push(Diagnostic::cast_error(
                    *loc,
                    format!("expected '{}', found integer", resolve_to.to_string(ns)),
                ));
                return Err(());
            } else {
                return Ok(Expression::NumberLiteral {
                    loc: *loc,
                    ty: resolve_to.clone(),
                    value: n.clone(),
                });
            }
        }
    }

    // Return smallest type

    let int_size = if let Some(size) = hex_str_len {
        (size * 4) as u64 // n bits = str_len / 2 * 8 = str_len * 4
    } else if bits < 7 {
        8
    } else {
        (bits + 7) & !7
    } as u16;

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
