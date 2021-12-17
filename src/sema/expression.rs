use num_bigint::BigInt;
use num_bigint::Sign;
use num_traits::FromPrimitive;
use num_traits::Num;
use num_traits::One;
use num_traits::Pow;
use num_traits::ToPrimitive;
use num_traits::Zero;
use std::cmp;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::collections::{BTreeMap, HashMap};
use std::ops::{Add, Shl, Sub};

use super::address::to_hexstr_eip55;
use super::ast::{
    Builtin, CallTy, Diagnostic, Expression, Function, Mutability, Namespace, StringLocation,
    Symbol, Type,
};
use super::builtin;
use super::contracts::{is_base, visit_bases};
use super::eval::eval_const_number;
use super::eval::eval_const_rational;
use super::format::string_format;
use super::symtable::Symtable;
use crate::parser::pt;
use crate::sema::unused_variable::{
    assigned_variable, check_function_call, check_var_usage_expression, used_variable,
};
use crate::Target;
use base58::{FromBase58, FromBase58Error};
use num_rational::BigRational;

impl Expression {
    /// Return the type for this expression. This assumes the expression has a single value,
    /// panics will occur otherwise
    pub fn ty(&self) -> Type {
        match self {
            Expression::BoolLiteral(_, _)
            | Expression::More(_, _, _)
            | Expression::Less(_, _, _)
            | Expression::MoreEqual(_, _, _)
            | Expression::LessEqual(_, _, _)
            | Expression::Equal(_, _, _)
            | Expression::Or(_, _, _)
            | Expression::And(_, _, _)
            | Expression::NotEqual(_, _, _)
            | Expression::Not(_, _)
            | Expression::StringCompare(_, _, _) => Type::Bool,
            Expression::AbiEncode { .. } | Expression::CodeLiteral(_, _, _) => Type::DynamicBytes,
            Expression::StringConcat(_, ty, _, _)
            | Expression::FunctionArg(_, ty, _)
            | Expression::BytesLiteral(_, ty, _)
            | Expression::NumberLiteral(_, ty, _)
            | Expression::RationalNumberLiteral(_, ty, _)
            | Expression::StructLiteral(_, ty, _)
            | Expression::ArrayLiteral(_, ty, _, _)
            | Expression::ConstArrayLiteral(_, ty, _, _)
            | Expression::Add(_, ty, ..)
            | Expression::Subtract(_, ty, ..)
            | Expression::Multiply(_, ty, _, _, _)
            | Expression::Divide(_, ty, _, _)
            | Expression::Modulo(_, ty, _, _)
            | Expression::Power(_, ty, _, _, _)
            | Expression::BitwiseOr(_, ty, _, _)
            | Expression::BitwiseAnd(_, ty, _, _)
            | Expression::BitwiseXor(_, ty, _, _)
            | Expression::ShiftLeft(_, ty, _, _)
            | Expression::ShiftRight(_, ty, _, _, _)
            | Expression::Variable(_, ty, _)
            | Expression::ConstantVariable(_, ty, _, _)
            | Expression::StorageVariable(_, ty, _, _)
            | Expression::Load(_, ty, _)
            | Expression::StorageLoad(_, ty, _)
            | Expression::ZeroExt(_, ty, _)
            | Expression::SignExt(_, ty, _)
            | Expression::Trunc(_, ty, _)
            | Expression::Cast(_, ty, _)
            | Expression::BytesCast(_, _, ty, _)
            | Expression::Complement(_, ty, _)
            | Expression::UnaryMinus(_, ty, _)
            | Expression::Ternary(_, ty, _, _, _)
            | Expression::StructMember(_, ty, _, _)
            | Expression::AllocDynamicArray(_, ty, _, _)
            | Expression::DynamicArraySubscript(_, ty, _, _)
            | Expression::PreIncrement(_, ty, ..)
            | Expression::PreDecrement(_, ty, ..)
            | Expression::PostIncrement(_, ty, ..)
            | Expression::PostDecrement(_, ty, ..)
            | Expression::Keccak256(_, ty, _)
            | Expression::Assign(_, ty, _, _) => ty.clone(),
            Expression::DynamicArrayPush(_, _, ty, _) | Expression::DynamicArrayPop(_, _, ty) => {
                match ty {
                    Type::Array(..) => ty.array_elem(),
                    Type::DynamicBytes => Type::Uint(8),
                    _ => unreachable!(),
                }
            }
            Expression::Subscript(_, ty, _, _) if ty.is_contract_storage() => {
                ty.storage_array_elem()
            }
            Expression::Subscript(_, ty, _, _) => ty.array_deref(),
            Expression::DynamicArrayLength(_, _) => Type::Uint(32),
            Expression::StorageArrayLength { ty, .. } => ty.clone(),
            Expression::StorageBytesSubscript(_, _, _) => {
                Type::StorageRef(false, Box::new(Type::Bytes(1)))
            }
            Expression::ExternalFunctionCallRaw { .. } => {
                panic!("two return values");
            }
            Expression::Builtin(_, returns, _, _)
            | Expression::InternalFunctionCall { returns, .. }
            | Expression::ExternalFunctionCall { returns, .. } => {
                assert_eq!(returns.len(), 1);
                returns[0].clone()
            }
            Expression::List(_, list) => {
                assert_eq!(list.len(), 1);

                list[0].ty()
            }
            Expression::Constructor { contract_no, .. } => Type::Contract(*contract_no),
            Expression::Poison => unreachable!(),
            Expression::InterfaceId(..) => Type::Bytes(4),
            Expression::FormatString(_, _) => Type::String,
            // codegen Expressions
            Expression::ReturnData(_) => Type::DynamicBytes,
            Expression::InternalFunction { ty, .. } => ty.clone(),
            Expression::ExternalFunction { ty, .. } => ty.clone(),
            Expression::InternalFunctionCfg(_) | Expression::Undefined(_) => unreachable!(),
        }
    }

    /// Is this expression 0
    fn const_zero(&self, contract_no: Option<usize>, ns: &Namespace) -> bool {
        if let Ok((_, value)) = eval_const_number(self, contract_no, ns) {
            value == BigInt::zero()
        } else {
            false
        }
    }

    /// Return the type for this expression.
    pub fn tys(&self) -> Vec<Type> {
        match self {
            Expression::Builtin(_, returns, _, _)
            | Expression::InternalFunctionCall { returns, .. }
            | Expression::ExternalFunctionCall { returns, .. } => returns.to_vec(),
            Expression::List(_, list) => list.iter().map(|e| e.ty()).collect(),
            Expression::ExternalFunctionCallRaw { .. } => vec![Type::Bool, Type::DynamicBytes],
            _ => vec![self.ty()],
        }
    }
}

/// Unescape a string literal
fn unescape(
    literal: &str,
    start: usize,
    file_no: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> String {
    let mut s = String::new();
    let mut indeces = literal.char_indices();

    while let Some((_, ch)) = indeces.next() {
        if ch != '\\' {
            s.push(ch);
            continue;
        }

        match indeces.next() {
            Some((_, '\n')) => (),
            Some((_, '\\')) => s.push('\\'),
            Some((_, '\'')) => s.push('\''),
            Some((_, '"')) => s.push('"'),
            Some((_, 'b')) => s.push('\u{0008}'),
            Some((_, 'f')) => s.push('\u{000c}'),
            Some((_, 'n')) => s.push('\n'),
            Some((_, 'r')) => s.push('\r'),
            Some((_, 't')) => s.push('\t'),
            Some((_, 'v')) => s.push('\u{000b}'),
            Some((i, 'x')) => match get_digits(&mut indeces, 2) {
                Ok(ch) => match std::char::from_u32(ch) {
                    Some(ch) => s.push(ch),
                    None => {
                        diagnostics.push(Diagnostic::error(
                            pt::Loc(file_no, start + i, start + i + 4),
                            format!("\\x{:02x} is not a valid unicode character", ch),
                        ));
                    }
                },
                Err(offset) => {
                    diagnostics.push(Diagnostic::error(
                        pt::Loc(
                            file_no,
                            start + i,
                            start + std::cmp::min(literal.len(), offset),
                        ),
                        "\\x escape should be followed by two hex digits".to_string(),
                    ));
                }
            },
            Some((i, 'u')) => match get_digits(&mut indeces, 4) {
                Ok(ch) => match std::char::from_u32(ch) {
                    Some(ch) => s.push(ch),
                    None => {
                        diagnostics.push(Diagnostic::error(
                            pt::Loc(file_no, start + i, start + i + 6),
                            format!("\\u{:04x} is not a valid unicode character", ch),
                        ));
                    }
                },
                Err(offset) => {
                    diagnostics.push(Diagnostic::error(
                        pt::Loc(
                            file_no,
                            start + i,
                            start + std::cmp::min(literal.len(), offset),
                        ),
                        "\\u escape should be followed by four hex digits".to_string(),
                    ));
                }
            },
            Some((i, ch)) => {
                diagnostics.push(Diagnostic::error(
                    pt::Loc(file_no, start + i, start + i + ch.len_utf8()),
                    format!("unknown escape character '{}'", ch),
                ));
            }
            None => unreachable!(),
        }
    }

    s
}

/// Get the hex digits for an escaped \x or \u. Returns either the value or
/// or the offset of the last character
fn get_digits(input: &mut std::str::CharIndices, len: usize) -> Result<u32, usize> {
    let mut n = 0;
    let offset;

    for _ in 0..len {
        if let Some((_, ch)) = input.next() {
            if let Some(v) = ch.to_digit(16) {
                n = (n << 4) + v;
                continue;
            }
            offset = match input.next() {
                Some((i, _)) => i,
                None => std::usize::MAX,
            };
        } else {
            offset = std::usize::MAX;
        }

        return Err(offset);
    }

    Ok(n)
}

fn coerce(
    l: &Type,
    l_loc: &pt::Loc,
    r: &Type,
    r_loc: &pt::Loc,
    ns: &Namespace,
    diagnostics: &mut Vec<Diagnostic>,
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

fn get_int_length(
    l: &Type,
    l_loc: &pt::Loc,
    allow_bytes: bool,
    ns: &Namespace,
    diagnostics: &mut Vec<Diagnostic>,
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
        Type::Struct(n) => {
            diagnostics.push(Diagnostic::error(
                *l_loc,
                format!("type struct {} not allowed", ns.structs[*n]),
            ));
            Err(())
        }
        Type::Array(_, _) => {
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

fn coerce_number(
    l: &Type,
    l_loc: &pt::Loc,
    r: &Type,
    r_loc: &pt::Loc,
    allow_bytes: bool,
    for_compare: bool,
    ns: &Namespace,
    diagnostics: &mut Vec<Diagnostic>,
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
        (Type::Bytes(left_length), Type::Bytes(right_length)) if allow_bytes => {
            return Ok(Type::Bytes(std::cmp::max(*left_length, *right_length)));
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
        _ => (),
    }

    let (left_len, left_signed) = get_int_length(l, l_loc, false, ns, diagnostics)?;

    let (right_len, right_signed) = get_int_length(r, r_loc, false, ns, diagnostics)?;

    Ok(match (left_signed, right_signed) {
        (true, true) => Type::Int(cmp::max(left_len, right_len)),
        (false, false) => Type::Uint(cmp::max(left_len, right_len)),
        (true, false) => Type::Int(cmp::max(left_len, cmp::min(right_len + 8, 256))),
        (false, true) => Type::Int(cmp::max(cmp::min(left_len + 8, 256), right_len)),
    })
}

/// Try to convert a BigInt into a Expression::NumberLiteral. This checks for sign,
/// width and creates to correct Type.
pub fn bigint_to_expression(
    loc: &pt::Loc,
    n: &BigInt,
    ns: &Namespace,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let bits = n.bits();

    if let ResolveTo::Type(resolve_to) = resolve_to {
        if !resolve_to.is_integer() {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!("expected ‘{}’, found integer", resolve_to.to_string(ns)),
            ));
            return Err(());
        }

        let permitted_bits = if resolve_to.is_signed_int() {
            resolve_to.bits(ns) as u64 - 1
        } else {
            resolve_to.bits(ns) as u64
        };

        return if n.sign() == Sign::Minus {
            if !resolve_to.is_signed_int() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "negative literal {} not allowed for unsigned type ‘{}’",
                        n,
                        resolve_to.to_string(ns)
                    ),
                ));
                Err(())
            } else if n.add(1u32).bits() > permitted_bits {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "literal {} is too large to fit into type ‘{}’",
                        n,
                        resolve_to.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::NumberLiteral(
                    *loc,
                    resolve_to.clone(),
                    n.clone(),
                ))
            }
        } else if bits > permitted_bits {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "literal {} is too large to fit into type ‘{}’",
                    n,
                    resolve_to.to_string(ns)
                ),
            ));
            Err(())
        } else {
            Ok(Expression::NumberLiteral(
                *loc,
                resolve_to.clone(),
                n.clone(),
            ))
        };
    }

    // Return smallest type

    let int_size = if bits < 7 { 8 } else { (bits + 7) & !7 } as u16;

    if n.sign() == Sign::Minus {
        if bits > 255 {
            diagnostics.push(Diagnostic::error(*loc, format!("{} is too large", n)));
            Err(())
        } else {
            Ok(Expression::NumberLiteral(
                *loc,
                Type::Int(int_size),
                n.clone(),
            ))
        }
    } else if bits > 256 {
        diagnostics.push(Diagnostic::error(*loc, format!("{} is too large", n)));
        Err(())
    } else {
        Ok(Expression::NumberLiteral(
            *loc,
            Type::Uint(int_size),
            n.clone(),
        ))
    }
}

/// Try to convert a Bigfloat into a Expression::RationalNumberLiteral. This checks for sign,
/// width and creates to correct Type.
pub fn bigdecimal_to_expression(
    loc: &pt::Loc,
    n: &BigRational,
    ns: &Namespace,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    if let ResolveTo::Type(resolve_to) = resolve_to {
        if !resolve_to.is_rational() {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!("expected ‘{}’, found rational", resolve_to.to_string(ns)),
            ));
            return Err(());
        } else {
            return Ok(Expression::RationalNumberLiteral(
                *loc,
                resolve_to.clone(),
                n.clone(),
            ));
        };
    }
    Err(())
}

/// Cast from one type to another, which also automatically derefs any Type::Ref() type.
/// if the cast is explicit (e.g. bytes32(bar) then implicit should be set to false.
pub fn cast(
    loc: &pt::Loc,
    expr: Expression,
    to: &Type,
    implicit: bool,
    ns: &Namespace,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let from = expr.ty();

    if &from == to {
        return Ok(expr);
    }

    // First of all, if we have a ref then derefence it
    if let Type::Ref(r) = from {
        return cast(
            loc,
            Expression::Load(*loc, r.as_ref().clone(), Box::new(expr)),
            to,
            implicit,
            ns,
            diagnostics,
        );
    }

    // If it's a storage reference then load the value. The expr is the storage slot
    if let Type::StorageRef(_, r) = from {
        if let Expression::StorageBytesSubscript(_, _, _) = expr {
            return Ok(expr);
        } else {
            return cast(
                loc,
                Expression::StorageLoad(*loc, *r, Box::new(expr)),
                to,
                implicit,
                ns,
                diagnostics,
            );
        }
    }

    // Special case: when converting literal sign can change if it fits
    match (&expr, &from, to) {
        (&Expression::NumberLiteral(_, _, ref n), p, &Type::Uint(to_len)) if p.is_primitive() => {
            return if n.sign() == Sign::Minus {
                if implicit {
                    diagnostics.push(Diagnostic::type_error(
                        *loc,
                        format!(
                            "implicit conversion cannot change negative number to ‘{}’",
                            to.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    // Convert to little endian so most significant bytes are at the end; that way
                    // we can simply resize the vector to the right size
                    let mut bs = n.to_signed_bytes_le();

                    bs.resize(to_len as usize / 8, 0xff);
                    Ok(Expression::NumberLiteral(
                        *loc,
                        Type::Uint(to_len),
                        BigInt::from_bytes_le(Sign::Plus, &bs),
                    ))
                }
            } else if n.bits() >= to_len as u64 {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion would truncate from ‘{}’ to ‘{}’",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::NumberLiteral(
                    *loc,
                    Type::Uint(to_len),
                    n.clone(),
                ))
            };
        }
        (&Expression::NumberLiteral(_, _, ref n), p, &Type::Int(to_len)) if p.is_primitive() => {
            return if n.bits() >= to_len as u64 {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion would truncate from ‘{}’ to ‘{}’",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::NumberLiteral(
                    *loc,
                    Type::Int(to_len),
                    n.clone(),
                ))
            };
        }
        (&Expression::NumberLiteral(_, _, ref n), p, &Type::Bytes(to_len)) if p.is_primitive() => {
            // round up the number of bits to bytes
            let bytes = (n.bits() + 7) / 8;
            return if n.sign() == Sign::Minus {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "negative number cannot be converted to type ‘{}’",
                        to.to_string(ns)
                    ),
                ));
                Err(())
            } else if n.sign() == Sign::Plus && bytes != to_len as u64 {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "number of {} bytes cannot be converted to type ‘{}’",
                        bytes,
                        to.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::NumberLiteral(
                    *loc,
                    Type::Bytes(to_len),
                    n.clone(),
                ))
            };
        }
        // Literal strings can be implicitly lengthened
        (&Expression::BytesLiteral(_, _, ref bs), p, &Type::Bytes(to_len)) if p.is_primitive() => {
            return if bs.len() > to_len as usize && implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion would truncate from ‘{}’ to ‘{}’",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                let mut bs = bs.to_owned();

                // Add zero's at the end as needed
                bs.resize(to_len as usize, 0);

                Ok(Expression::BytesLiteral(*loc, Type::Bytes(to_len), bs))
            };
        }
        (&Expression::BytesLiteral(loc, _, ref init), _, &Type::DynamicBytes)
        | (&Expression::BytesLiteral(loc, _, ref init), _, &Type::String) => {
            return Ok(Expression::AllocDynamicArray(
                loc,
                to.clone(),
                Box::new(Expression::NumberLiteral(
                    loc,
                    Type::Uint(32),
                    BigInt::from(init.len()),
                )),
                Some(init.clone()),
            ));
        }
        (&Expression::NumberLiteral(_, _, ref n), _, &Type::Rational) => {
            return Ok(Expression::RationalNumberLiteral(
                *loc,
                Type::Rational,
                BigRational::from(n.clone()),
            ));
        }
        _ => (),
    };

    cast_types(loc, expr, from, to.clone(), implicit, ns, diagnostics)
}

/// Do casting between types (no literals)
fn cast_types(
    loc: &pt::Loc,
    expr: Expression,
    from: Type,
    to: Type,
    implicit: bool,
    ns: &Namespace,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let address_bits = ns.address_length as u16 * 8;

    #[allow(clippy::comparison_chain)]
    match (&from, &to) {
        (Type::Uint(from_width), Type::Enum(enum_no))
        | (Type::Int(from_width), Type::Enum(enum_no)) => {
            if implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion from {} to {} not allowed",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));
                return Err(());
            }

            let enum_ty = &ns.enums[*enum_no];

            // TODO would be help to have current contract to resolve contract constants
            if let Ok((_, big_number)) = eval_const_number(&expr, None, ns) {
                if let Some(number) = big_number.to_usize() {
                    if enum_ty.values.values().any(|(_, v)| *v == number) {
                        return Ok(Expression::NumberLiteral(
                            expr.loc(),
                            to.clone(),
                            big_number,
                        ));
                    }
                }

                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "enum {} has no value with ordinal {}",
                        to.to_string(ns),
                        big_number
                    ),
                ));
                return Err(());
            }

            let to_width = enum_ty.ty.bits(ns);

            // TODO needs runtime checks
            match from_width.cmp(&to_width) {
                Ordering::Greater => Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr))),
                Ordering::Less => Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr))),
                Ordering::Equal => Ok(Expression::Cast(*loc, to.clone(), Box::new(expr))),
            }
        }
        (Type::Enum(enum_no), Type::Uint(to_width))
        | (Type::Enum(enum_no), Type::Int(to_width)) => {
            if implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion from {} to {} not allowed",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));
                return Err(());
            }

            let enum_ty = &ns.enums[*enum_no];
            let from_width = enum_ty.ty.bits(ns);

            match from_width.cmp(to_width) {
                Ordering::Greater => Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr))),
                Ordering::Less => Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr))),
                Ordering::Equal => Ok(Expression::Cast(*loc, to.clone(), Box::new(expr))),
            }
        }
        (Type::Bytes(1), Type::Uint(8)) => Ok(expr),
        (Type::Uint(8), Type::Bytes(1)) => Ok(expr),
        (Type::Uint(from_len), Type::Uint(to_len)) => match from_len.cmp(to_len) {
            Ordering::Greater => {
                if implicit {
                    diagnostics.push(Diagnostic::type_error(
                        *loc,
                        format!(
                            "implicit conversion would truncate from {} to {}",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
                }
            }
            Ordering::Less => Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr))),
            Ordering::Equal => Ok(Expression::Cast(*loc, to.clone(), Box::new(expr))),
        },
        (Type::Int(from_len), Type::Int(to_len)) => match from_len.cmp(to_len) {
            Ordering::Greater => {
                if implicit {
                    diagnostics.push(Diagnostic::type_error(
                        *loc,
                        format!(
                            "implicit conversion would truncate from {} to {}",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
                }
            }
            Ordering::Less => Ok(Expression::SignExt(*loc, to.clone(), Box::new(expr))),
            Ordering::Equal => Ok(Expression::Cast(*loc, to.clone(), Box::new(expr))),
        },
        (Type::Uint(from_len), Type::Int(to_len)) if to_len > from_len => {
            Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr)))
        }
        (Type::Int(from_len), Type::Uint(to_len)) => {
            if implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion would change sign from {} to {}",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));
                Err(())
            } else if from_len > to_len {
                Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
            } else if from_len < to_len {
                Ok(Expression::SignExt(*loc, to.clone(), Box::new(expr)))
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        (Type::Uint(from_len), Type::Int(to_len)) => {
            if implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion would change sign from {} to {}",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));
                Err(())
            } else if from_len > to_len {
                Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
            } else if from_len < to_len {
                Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr)))
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        // Casting value to uint
        (Type::Value, Type::Uint(to_len)) => {
            let from_len = ns.value_length * 8;
            let to_len = *to_len as usize;

            match from_len.cmp(&to_len) {
                Ordering::Greater => {
                    if implicit {
                        diagnostics.push(Diagnostic::type_error(
                            *loc,
                            format!(
                                "implicit conversion would truncate from {} to {}",
                                from.to_string(ns),
                                to.to_string(ns)
                            ),
                        ));
                        Err(())
                    } else {
                        Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
                    }
                }
                Ordering::Less => Ok(Expression::SignExt(*loc, to.clone(), Box::new(expr))),
                Ordering::Equal => Ok(Expression::Cast(*loc, to.clone(), Box::new(expr))),
            }
        }
        (Type::Value, Type::Int(to_len)) => {
            let from_len = ns.value_length * 8;
            let to_len = *to_len as usize;

            if implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion would change sign from {} to {}",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));
                Err(())
            } else if from_len > to_len {
                Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
            } else if from_len < to_len {
                Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr)))
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        // Casting value to uint
        (Type::Uint(from_len), Type::Value) => {
            let from_len = *from_len as usize;
            let to_len = ns.value_length * 8;

            match from_len.cmp(&to_len) {
                Ordering::Greater => {
                    if implicit {
                        diagnostics.push(Diagnostic::type_error(
                            *loc,
                            format!(
                                "conversion truncates {} to {}, as value is type {} on target {}",
                                from.to_string(ns),
                                to.to_string(ns),
                                Type::Value.to_string(ns),
                                ns.target
                            ),
                        ));
                        Err(())
                    } else {
                        Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
                    }
                }
                Ordering::Less => Ok(Expression::SignExt(*loc, to.clone(), Box::new(expr))),
                Ordering::Equal => Ok(Expression::Cast(*loc, to.clone(), Box::new(expr))),
            }
        }
        // Casting int to address
        (Type::Uint(from_len), Type::Address(_)) | (Type::Int(from_len), Type::Address(_)) => {
            if implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion from {} to address not allowed",
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if *from_len > address_bits {
                Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
            } else if *from_len < address_bits {
                Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr)))
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        // Casting int address to int
        (Type::Address(_), Type::Uint(to_len)) | (Type::Address(_), Type::Int(to_len)) => {
            if implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));
                Err(())
            } else if *to_len < address_bits {
                Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
            } else if *to_len > address_bits {
                Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr)))
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        // Lengthing or shorting a fixed bytes array
        (Type::Bytes(from_len), Type::Bytes(to_len)) => {
            if implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion would truncate from {} to {}",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));
                Err(())
            } else if to_len > from_len {
                let shift = (to_len - from_len) * 8;

                Ok(Expression::ShiftLeft(
                    *loc,
                    to.clone(),
                    Box::new(Expression::ZeroExt(*loc, to.clone(), Box::new(expr))),
                    Box::new(Expression::NumberLiteral(
                        *loc,
                        Type::Uint(*to_len as u16 * 8),
                        BigInt::from_u8(shift).unwrap(),
                    )),
                ))
            } else {
                let shift = (from_len - to_len) * 8;

                Ok(Expression::Trunc(
                    *loc,
                    to.clone(),
                    Box::new(Expression::ShiftRight(
                        *loc,
                        from.clone(),
                        Box::new(expr),
                        Box::new(Expression::NumberLiteral(
                            *loc,
                            Type::Uint(*from_len as u16 * 8),
                            BigInt::from_u8(shift).unwrap(),
                        )),
                        false,
                    )),
                ))
            }
        }
        (Type::Rational, Type::Uint(_) | Type::Int(_) | Type::Value) => {
            match eval_const_rational(&expr, None, ns) {
                Ok((_, big_number)) => {
                    if big_number.is_integer() {
                        let expr = Expression::NumberLiteral(
                            expr.loc(),
                            to.clone(),
                            big_number.to_integer(),
                        );

                        return cast(loc, expr, &to, true, ns, diagnostics);
                    }

                    diagnostics.push(Diagnostic::type_error(
                        *loc,
                        format!(
                            "conversion to {} from {} not allowed",
                            to.to_string(ns),
                            from.to_string(ns)
                        ),
                    ));

                    Err(())
                }
                Err(diag) => {
                    diagnostics.push(diag);
                    Err(())
                }
            }
        }
        (Type::Uint(_) | Type::Int(_) | Type::Value, Type::Rational) => {
            Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
        }
        (Type::Bytes(_), Type::DynamicBytes) | (Type::DynamicBytes, Type::Bytes(_)) => Ok(
            Expression::BytesCast(*loc, from.clone(), to.clone(), Box::new(expr)),
        ),
        // Explicit conversion from bytesN to int/uint only allowed with expliciy
        // cast and if it is the same size (i.e. no conversion required)
        (Type::Bytes(from_len), Type::Uint(to_len))
        | (Type::Bytes(from_len), Type::Int(to_len)) => {
            if implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if *from_len as u16 * 8 != *to_len {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        // Explicit conversion to bytesN from int/uint only allowed with expliciy
        // cast and if it is the same size (i.e. no conversion required)
        (Type::Uint(from_len), Type::Bytes(to_len))
        | (Type::Int(from_len), Type::Bytes(to_len)) => {
            if implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if *to_len as u16 * 8 != *from_len {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        // Explicit conversion from bytesN to address only allowed with expliciy
        // cast and if it is the same size (i.e. no conversion required)
        (Type::Bytes(from_len), Type::Address(_)) => {
            if implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if *from_len as usize != ns.address_length {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        // Explicit conversion between contract and address is allowed
        (Type::Address(false), Type::Address(true))
        | (Type::Address(_), Type::Contract(_))
        | (Type::Contract(_), Type::Address(_)) => {
            if implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        // Conversion between contracts is allowed if it is a base
        (Type::Contract(contract_no_from), Type::Contract(contract_no_to)) => {
            if implicit && !is_base(*contract_no_to, *contract_no_from, ns) {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion not allowed since {} is not a base contract of {}",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        // conversion from address payable to address is implicitly allowed (not vice versa)
        (Type::Address(true), Type::Address(false)) => {
            Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
        }
        // Explicit conversion to bytesN from int/uint only allowed with expliciy
        // cast and if it is the same size (i.e. no conversion required)
        (Type::Address(_), Type::Bytes(to_len)) => {
            if implicit {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if *to_len as usize != ns.address_length {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        (Type::String, Type::DynamicBytes) | (Type::DynamicBytes, Type::String) if !implicit => {
            Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
        }
        // string conversions
        // (Type::Bytes(_), Type::String) => Ok(Expression::Cast(*loc, to.clone(), Box::new(expr))),
        /*
        (Type::String, Type::Bytes(to_len)) => {
            if let Expression::BytesLiteral(_, from_str) = &expr {
                if from_str.len() > to_len as usize {
                    diagnostics.push(Output::type_error(
                        *loc,
                        format!(
                            "string of {} bytes is too long to fit into {}",
                            from_str.len(),
                            to.to_string(ns)
                        ),
                    ));
                    return Err(());
                }
            }
            Ok(Expression::Cast(*loc, to.clone(), Box::new(expr))
        }
        */
        (Type::Void, _) => {
            diagnostics.push(Diagnostic::type_error(
                *loc,
                "function or method does not return a value".to_string(),
            ));
            Err(())
        }
        (
            Type::ExternalFunction {
                params: from_params,
                mutability: from_mutablity,
                returns: from_returns,
            },
            Type::ExternalFunction {
                params: to_params,
                mutability: to_mutablity,
                returns: to_returns,
            },
        )
        | (
            Type::InternalFunction {
                params: from_params,
                mutability: from_mutablity,
                returns: from_returns,
            },
            Type::InternalFunction {
                params: to_params,
                mutability: to_mutablity,
                returns: to_returns,
            },
        ) => {
            if from_params != to_params {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "function arguments do not match in conversion from ‘{}’ to ‘{}’",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if from_returns != to_returns {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "function returns do not match in conversion from ‘{}’ to ‘{}’",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if !compatible_mutability(from_mutablity, to_mutablity) {
                diagnostics.push(Diagnostic::type_error(
                    *loc,
                    format!(
                        "function mutability not compatible in conversion from ‘{}’ to ‘{}’",
                        from.to_string(ns),
                        to.to_string(ns),
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        _ => {
            diagnostics.push(Diagnostic::type_error(
                *loc,
                format!(
                    "conversion from {} to {} not possible",
                    from.to_string(ns),
                    to.to_string(ns)
                ),
            ));
            Err(())
        }
    }
}

/// Compare two mutability levels
pub fn compatible_mutability(left: &Mutability, right: &Mutability) -> bool {
    matches!(
        (left, right),
        // only payable is compatible with payable
        (Mutability::Payable(_), Mutability::Payable(_))
            // default is compatible with anything but pure and view
            | (Mutability::Nonpayable(_), Mutability::Nonpayable(_) | Mutability::Payable(_))
            // view is compatible with anything but pure
            | (Mutability::View(_), Mutability::View(_) | Mutability::Nonpayable(_) | Mutability::Payable(_))
            // pure is compatible with anything
            | (Mutability::Pure(_), _) // everything else is not compatible
    )
}

/// When resolving an expression, what type are we looking for
#[derive(PartialEq, Clone, Copy)]
pub enum ResolveTo<'a> {
    Unknown,        // We don't know what we're looking for, best effort
    Discard,        // We won't be using the result. For example, an expression as a statement/
    Type(&'a Type), // We will be wanting this type please, e.g. `int64 x = 1;`
}

#[derive(Clone)]
pub struct ExprContext {
    /// What source file are we in
    pub file_no: usize,
    // Are we resolving a contract, and if so, which one
    pub contract_no: Option<usize>,
    // Are resolving the body of a function, and if os, which one
    pub function_no: Option<usize>,
    // Are we currently in an unchecked block
    pub unchecked: bool,
    // Are we evaluating a constant expression
    pub constant: bool,
}

/// Resolve a parsed expression into an AST expression. The resolve_to argument is a hint to what
/// type the result should be.
pub fn expression(
    expr: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    match expr {
        pt::Expression::ArrayLiteral(loc, exprs) => {
            let res =
                resolve_array_literal(loc, exprs, context, ns, symtable, diagnostics, resolve_to);

            if let Ok(exp) = &res {
                used_variable(ns, exp, symtable);
            }

            res
        }
        pt::Expression::BoolLiteral(loc, v) => Ok(Expression::BoolLiteral(*loc, *v)),
        pt::Expression::StringLiteral(v) => {
            Ok(string_literal(v, context.file_no, diagnostics, resolve_to))
        }
        pt::Expression::HexLiteral(v) => hex_literal(v, diagnostics),
        pt::Expression::NumberLiteral(loc, b) => {
            bigint_to_expression(loc, b, ns, diagnostics, resolve_to)
        }
        pt::Expression::RationalNumberLiteral(loc, b) => Ok(Expression::RationalNumberLiteral(
            *loc,
            Type::Rational,
            b.clone(),
        )),
        pt::Expression::HexNumberLiteral(loc, n) => {
            hex_number_literal(loc, n, ns, diagnostics, resolve_to)
        }
        pt::Expression::AddressLiteral(loc, address) => {
            address_literal(loc, address, ns, diagnostics)
        }
        pt::Expression::Variable(id) => {
            variable(id, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::Add(loc, l, r) => {
            addition(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::Subtract(loc, l, r) => {
            subtract(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::BitwiseOr(loc, l, r) => {
            bitwise_or(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::BitwiseAnd(loc, l, r) => {
            bitwise_and(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::BitwiseXor(loc, l, r) => {
            bitwise_xor(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::ShiftLeft(loc, l, r) => {
            shift_left(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::ShiftRight(loc, l, r) => {
            shift_right(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::Multiply(loc, l, r) => {
            multiply(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::Divide(loc, l, r) => {
            divide(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::Modulo(loc, l, r) => {
            modulo(loc, l, r, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::Power(loc, b, e) => {
            power(loc, b, e, context, ns, symtable, diagnostics, resolve_to)
        }
        // compare
        pt::Expression::More(loc, l, r) => {
            let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
            let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;

            check_var_usage_expression(ns, &left, &right, symtable);
            let ty = coerce_number(
                &left.ty(),
                &l.loc(),
                &right.ty(),
                &r.loc(),
                true,
                true,
                ns,
                diagnostics,
            )?;

            Ok(Expression::More(
                *loc,
                Box::new(cast(&l.loc(), left, &ty, true, ns, diagnostics)?),
                Box::new(cast(&r.loc(), right, &ty, true, ns, diagnostics)?),
            ))
        }
        pt::Expression::Less(loc, l, r) => {
            let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
            let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;

            check_var_usage_expression(ns, &left, &right, symtable);

            let ty = coerce_number(
                &left.ty(),
                &l.loc(),
                &right.ty(),
                &r.loc(),
                true,
                true,
                ns,
                diagnostics,
            )?;

            Ok(Expression::Less(
                *loc,
                Box::new(cast(&l.loc(), left, &ty, true, ns, diagnostics)?),
                Box::new(cast(&r.loc(), right, &ty, true, ns, diagnostics)?),
            ))
        }
        pt::Expression::MoreEqual(loc, l, r) => {
            let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
            let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
            check_var_usage_expression(ns, &left, &right, symtable);

            let ty = coerce_number(
                &left.ty(),
                &l.loc(),
                &right.ty(),
                &r.loc(),
                true,
                true,
                ns,
                diagnostics,
            )?;

            Ok(Expression::MoreEqual(
                *loc,
                Box::new(cast(&l.loc(), left, &ty, true, ns, diagnostics)?),
                Box::new(cast(&r.loc(), right, &ty, true, ns, diagnostics)?),
            ))
        }
        pt::Expression::LessEqual(loc, l, r) => {
            let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
            let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
            check_var_usage_expression(ns, &left, &right, symtable);

            let ty = coerce_number(
                &left.ty(),
                &l.loc(),
                &right.ty(),
                &r.loc(),
                true,
                true,
                ns,
                diagnostics,
            )?;

            Ok(Expression::LessEqual(
                *loc,
                Box::new(cast(&l.loc(), left, &ty, true, ns, diagnostics)?),
                Box::new(cast(&r.loc(), right, &ty, true, ns, diagnostics)?),
            ))
        }
        pt::Expression::Equal(loc, l, r) => equal(loc, l, r, context, ns, symtable, diagnostics),

        pt::Expression::NotEqual(loc, l, r) => Ok(Expression::Not(
            *loc,
            Box::new(equal(loc, l, r, context, ns, symtable, diagnostics)?),
        )),
        // unary expressions
        pt::Expression::Not(loc, e) => {
            let expr = expression(e, context, ns, symtable, diagnostics, resolve_to)?;

            used_variable(ns, &expr, symtable);
            Ok(Expression::Not(
                *loc,
                Box::new(cast(loc, expr, &Type::Bool, true, ns, diagnostics)?),
            ))
        }
        pt::Expression::Complement(loc, e) => {
            let expr = expression(e, context, ns, symtable, diagnostics, resolve_to)?;

            used_variable(ns, &expr, symtable);
            let expr_ty = expr.ty();

            get_int_length(&expr_ty, loc, true, ns, diagnostics)?;

            Ok(Expression::Complement(*loc, expr_ty, Box::new(expr)))
        }
        pt::Expression::UnaryMinus(loc, e) => match e.as_ref() {
            pt::Expression::NumberLiteral(_, n) => {
                bigint_to_expression(loc, &-n, ns, diagnostics, resolve_to)
            }
            pt::Expression::HexNumberLiteral(_, v) => {
                // a hex literal with a minus before it cannot be an address literal or a bytesN value
                let s: String = v.chars().skip(2).filter(|v| *v != '_').collect();

                let n = BigInt::from_str_radix(&s, 16).unwrap();

                bigint_to_expression(loc, &-n, ns, diagnostics, resolve_to)
            }
            pt::Expression::RationalNumberLiteral(_, r) => {
                bigdecimal_to_expression(loc, &-r, ns, diagnostics, resolve_to)
            }
            e => {
                let expr = expression(e, context, ns, symtable, diagnostics, resolve_to)?;

                used_variable(ns, &expr, symtable);
                let expr_type = expr.ty();

                if let Expression::NumberLiteral(_, _, n) = expr {
                    bigint_to_expression(loc, &-n, ns, diagnostics, resolve_to)
                } else if let Expression::RationalNumberLiteral(_, _, r) = expr {
                    bigdecimal_to_expression(loc, &-r, ns, diagnostics, resolve_to)
                } else {
                    get_int_length(&expr_type, loc, false, ns, diagnostics)?;

                    Ok(Expression::UnaryMinus(*loc, expr_type, Box::new(expr)))
                }
            }
        },
        pt::Expression::UnaryPlus(loc, e) => {
            let expr = expression(e, context, ns, symtable, diagnostics, resolve_to)?;
            used_variable(ns, &expr, symtable);
            let expr_type = expr.ty();

            get_int_length(&expr_type, loc, false, ns, diagnostics)?;

            Ok(expr)
        }

        pt::Expression::Ternary(loc, c, l, r) => {
            let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
            let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;
            check_var_usage_expression(ns, &left, &right, symtable);
            let cond = expression(c, context, ns, symtable, diagnostics, resolve_to)?;
            used_variable(ns, &cond, symtable);

            let cond = cast(&c.loc(), cond, &Type::Bool, true, ns, diagnostics)?;

            let ty = coerce(&left.ty(), &l.loc(), &right.ty(), &r.loc(), ns, diagnostics)?;
            let left = cast(&l.loc(), left, &ty, true, ns, diagnostics)?;
            let right = cast(&r.loc(), right, &ty, true, ns, diagnostics)?;

            Ok(Expression::Ternary(
                *loc,
                ty,
                Box::new(cond),
                Box::new(left),
                Box::new(right),
            ))
        }

        // pre/post decrement/increment
        pt::Expression::PostIncrement(loc, var)
        | pt::Expression::PreIncrement(loc, var)
        | pt::Expression::PostDecrement(loc, var)
        | pt::Expression::PreDecrement(loc, var) => {
            if context.constant {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "operator not allowed in constant context".to_string(),
                ));
                return Err(());
            };

            incr_decr(var, expr, context, ns, symtable, diagnostics)
        }

        // assignment
        pt::Expression::Assign(loc, var, e) => {
            if context.constant {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "assignment not allowed in constant context".to_string(),
                ));
                return Err(());
            };

            assign_single(loc, var, e, context, ns, symtable, diagnostics)
        }

        pt::Expression::AssignAdd(loc, var, e)
        | pt::Expression::AssignSubtract(loc, var, e)
        | pt::Expression::AssignMultiply(loc, var, e)
        | pt::Expression::AssignDivide(loc, var, e)
        | pt::Expression::AssignModulo(loc, var, e)
        | pt::Expression::AssignOr(loc, var, e)
        | pt::Expression::AssignAnd(loc, var, e)
        | pt::Expression::AssignXor(loc, var, e)
        | pt::Expression::AssignShiftLeft(loc, var, e)
        | pt::Expression::AssignShiftRight(loc, var, e) => {
            if context.constant {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "assignment not allowed in constant context".to_string(),
                ));
                return Err(());
            };

            assign_expr(loc, var, expr, e, context, ns, symtable, diagnostics)
        }
        pt::Expression::NamedFunctionCall(loc, ty, args) => {
            named_call_expr(loc, ty, args, false, context, ns, symtable, diagnostics)
        }
        pt::Expression::New(loc, call) => {
            if context.constant {
                diagnostics.push(Diagnostic::error(
                    expr.loc(),
                    "new not allowed in constant expression".to_string(),
                ));
                return Err(());
            }

            match call.as_ref() {
                pt::Expression::FunctionCall(_, ty, args) => {
                    let res = new(loc, ty, args, context, ns, symtable, diagnostics);

                    if let Ok(exp) = &res {
                        check_function_call(ns, exp, symtable);
                    }
                    res
                }
                pt::Expression::NamedFunctionCall(_, ty, args) => {
                    let res =
                        constructor_named_args(loc, ty, args, context, ns, symtable, diagnostics);

                    if let Ok(exp) = &res {
                        check_function_call(ns, exp, symtable);
                    }

                    res
                }
                _ => unreachable!(),
            }
        }
        pt::Expression::Delete(loc, _) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "delete not allowed in expression".to_string(),
            ));
            Err(())
        }
        pt::Expression::FunctionCall(loc, ty, args) => call_expr(
            loc,
            ty,
            args,
            false,
            context,
            ns,
            symtable,
            diagnostics,
            resolve_to,
        ),
        pt::Expression::ArraySubscript(loc, _, None) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "expected expression before ‘]’ token".to_string(),
            ));

            Err(())
        }
        pt::Expression::ArraySubscript(loc, array, Some(index)) => {
            array_subscript(loc, array, index, context, ns, symtable, diagnostics)
        }
        pt::Expression::MemberAccess(loc, e, id) => {
            member_access(loc, e, id, context, ns, symtable, diagnostics, resolve_to)
        }
        pt::Expression::Or(loc, left, right) => {
            let boolty = Type::Bool;
            let l = cast(
                loc,
                expression(left, context, ns, symtable, diagnostics, resolve_to)?,
                &boolty,
                true,
                ns,
                diagnostics,
            )?;
            let r = cast(
                loc,
                expression(right, context, ns, symtable, diagnostics, resolve_to)?,
                &boolty,
                true,
                ns,
                diagnostics,
            )?;

            check_var_usage_expression(ns, &l, &r, symtable);

            Ok(Expression::Or(*loc, Box::new(l), Box::new(r)))
        }
        pt::Expression::And(loc, left, right) => {
            let boolty = Type::Bool;
            let l = cast(
                loc,
                expression(left, context, ns, symtable, diagnostics, resolve_to)?,
                &boolty,
                true,
                ns,
                diagnostics,
            )?;
            let r = cast(
                loc,
                expression(right, context, ns, symtable, diagnostics, resolve_to)?,
                &boolty,
                true,
                ns,
                diagnostics,
            )?;
            check_var_usage_expression(ns, &l, &r, symtable);

            Ok(Expression::And(*loc, Box::new(l), Box::new(r)))
        }
        pt::Expression::Type(loc, _) => {
            diagnostics.push(Diagnostic::error(*loc, "type not expected".to_owned()));
            Err(())
        }
        pt::Expression::List(loc, _) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "lists only permitted in destructure statements".to_owned(),
            ));
            Err(())
        }
        pt::Expression::FunctionCallBlock(loc, _, _) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "unexpect block encountered".to_owned(),
            ));
            Err(())
        }
        pt::Expression::Unit(loc, expr, unit) => {
            let n = match expr.as_ref() {
                pt::Expression::NumberLiteral(_, n) => n,
                pt::Expression::HexNumberLiteral(loc, _) => {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        "hexadecimal numbers cannot be used with unit denominations".to_owned(),
                    ));
                    return Err(());
                }
                _ => {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        "unit denominations can only be used with number literals".to_owned(),
                    ));
                    return Err(());
                }
            };

            match unit {
                pt::Unit::Wei(loc)
                | pt::Unit::Finney(loc)
                | pt::Unit::Szabo(loc)
                | pt::Unit::Ether(loc)
                    if ns.target != crate::Target::Ewasm =>
                {
                    diagnostics.push(Diagnostic::warning(
                        *loc,
                        "ethereum currency unit used while not targetting ethereum".to_owned(),
                    ));
                }
                _ => (),
            }

            bigint_to_expression(
                loc,
                &(n * match unit {
                    pt::Unit::Seconds(_) => BigInt::from(1),
                    pt::Unit::Minutes(_) => BigInt::from(60),
                    pt::Unit::Hours(_) => BigInt::from(60 * 60),
                    pt::Unit::Days(_) => BigInt::from(60 * 60 * 24),
                    pt::Unit::Weeks(_) => BigInt::from(60 * 60 * 24 * 7),
                    pt::Unit::Wei(_) => BigInt::from(1),
                    pt::Unit::Szabo(_) => BigInt::from(10).pow(12u32),
                    pt::Unit::Finney(_) => BigInt::from(10).pow(15u32),
                    pt::Unit::Ether(_) => BigInt::from(10).pow(18u32),
                }),
                ns,
                diagnostics,
                resolve_to,
            )
        }
        pt::Expression::This(loc) => match context.contract_no {
            Some(contract_no) => Ok(Expression::Builtin(
                *loc,
                vec![Type::Contract(contract_no)],
                Builtin::GetAddress,
                Vec::new(),
            )),
            None => {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "this not allowed outside contract".to_owned(),
                ));
                Err(())
            }
        },
    }
}

fn string_literal(
    v: &[pt::StringLiteral],
    file_no: usize,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Expression {
    // Concatenate the strings
    let mut result = Vec::new();
    let mut loc = v[0].loc;

    for s in v {
        result.extend_from_slice(unescape(&s.string, s.loc.1, file_no, diagnostics).as_bytes());
        loc.2 = s.loc.2;
    }

    let length = result.len();

    if let ResolveTo::Type(Type::String) = resolve_to {
        Expression::AllocDynamicArray(
            loc,
            Type::String,
            Box::new(Expression::NumberLiteral(
                loc,
                Type::Uint(32),
                BigInt::from(length),
            )),
            Some(result),
        )
    } else {
        Expression::BytesLiteral(loc, Type::Bytes(length as u8), result)
    }
}

fn hex_literal(v: &[pt::HexLiteral], diagnostics: &mut Vec<Diagnostic>) -> Result<Expression, ()> {
    let mut result = Vec::new();
    let mut loc = v[0].loc;

    for s in v {
        if (s.hex.len() % 2) != 0 {
            diagnostics.push(Diagnostic::error(
                s.loc,
                format!("hex string \"{}\" has odd number of characters", s.hex),
            ));
            return Err(());
        } else {
            result.extend_from_slice(&hex::decode(&s.hex).unwrap());
            loc.2 = s.loc.2;
        }
    }

    let length = result.len();

    Ok(Expression::BytesLiteral(
        loc,
        Type::Bytes(length as u8),
        result,
    ))
}

fn hex_number_literal(
    loc: &pt::Loc,
    n: &str,
    ns: &mut Namespace,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    // ns.address_length is in bytes; double for hex and two for the leading 0x
    if n.starts_with("0x") && !n.chars().any(|c| c == '_') && n.len() == 42 {
        let address = to_hexstr_eip55(n);

        if ns.target == Target::Ewasm {
            return if address == *n {
                let s: String = address.chars().skip(2).collect();

                Ok(Expression::NumberLiteral(
                    *loc,
                    Type::Address(false),
                    BigInt::from_str_radix(&s, 16).unwrap(),
                ))
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "address literal has incorrect checksum, expected ‘{}’",
                        address
                    ),
                ));
                Err(())
            };
        } else if address == *n {
            // looks like ethereum address
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "ethereum address literal ‘{}’ not supported on target {}",
                    n, ns.target
                ),
            ));
            return Err(());
        }
    }

    // from_str_radix does not like the 0x prefix
    let s: String = n.chars().skip(2).filter(|v| *v != '_').collect();

    // hex values are allowed for bytesN but the length must match
    if let ResolveTo::Type(Type::Bytes(length)) = resolve_to {
        let expected_length = *length as usize * 2;
        let val = BigInt::from_str_radix(&s, 16).unwrap();

        return if !val.is_zero() && s.len() != expected_length {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "hex literal {} must be {} digits for type ‘bytes{}’",
                    n, expected_length, length,
                ),
            ));
            Err(())
        } else {
            Ok(Expression::NumberLiteral(*loc, Type::Bytes(*length), val))
        };
    }

    bigint_to_expression(
        loc,
        &BigInt::from_str_radix(&s, 16).unwrap(),
        ns,
        diagnostics,
        resolve_to,
    )
}

fn address_literal(
    loc: &pt::Loc,
    address: &str,
    ns: &mut Namespace,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    if ns.target.is_substrate() {
        match address.from_base58() {
            Ok(v) => {
                if v.len() != ns.address_length + 3 {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "address literal {} incorrect length of {}",
                            address,
                            v.len()
                        ),
                    ));
                    return Err(());
                }

                let hash_data: Vec<u8> = b"SS58PRE"
                    .iter()
                    .chain(v[..=ns.address_length].iter())
                    .cloned()
                    .collect();

                let hash = blake2_rfc::blake2b::blake2b(64, &[], &hash_data);
                let hash = hash.as_bytes();

                if v[ns.address_length + 1] != hash[0] || v[ns.address_length + 2] != hash[1] {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!("address literal {} hash incorrect checksum", address,),
                    ));
                    return Err(());
                }

                Ok(Expression::NumberLiteral(
                    *loc,
                    Type::Address(false),
                    BigInt::from_bytes_be(Sign::Plus, &v[1..ns.address_length + 1]),
                ))
            }
            Err(FromBase58Error::InvalidBase58Length) => {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!("address literal {} invalid base58 length", address),
                ));
                Err(())
            }
            Err(FromBase58Error::InvalidBase58Character(ch, pos)) => {
                let mut loc = *loc;
                loc.1 += pos;
                loc.2 = loc.1;
                diagnostics.push(Diagnostic::error(
                    loc,
                    format!("address literal {} invalid character '{}'", address, ch),
                ));
                Err(())
            }
        }
    } else if ns.target == Target::Solana {
        match address.from_base58() {
            Ok(v) => {
                if v.len() != ns.address_length {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "address literal {} incorrect length of {}",
                            address,
                            v.len()
                        ),
                    ));
                    Err(())
                } else {
                    Ok(Expression::NumberLiteral(
                        *loc,
                        Type::Address(false),
                        BigInt::from_bytes_be(Sign::Plus, &v),
                    ))
                }
            }
            Err(FromBase58Error::InvalidBase58Length) => {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!("address literal {} invalid base58 length", address),
                ));
                Err(())
            }
            Err(FromBase58Error::InvalidBase58Character(ch, pos)) => {
                let mut loc = *loc;
                loc.1 += pos;
                loc.2 = loc.1;
                diagnostics.push(Diagnostic::error(
                    loc,
                    format!("address literal {} invalid character '{}'", address, ch),
                ));
                Err(())
            }
        }
    } else {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!("address literal {} not supported on {}", address, ns.target),
        ));
        Err(())
    }
}

fn variable(
    id: &pt::Identifier,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    if let Some(v) = symtable.find(&id.name) {
        return if context.constant {
            diagnostics.push(Diagnostic::error(
                id.loc,
                format!("cannot read variable ‘{}’ in constant expression", id.name),
            ));
            Err(())
        } else {
            Ok(Expression::Variable(id.loc, v.ty.clone(), v.pos))
        };
    }

    if let Some((builtin, ty)) = builtin::builtin_var(&id.loc, None, &id.name, ns, diagnostics) {
        return Ok(Expression::Builtin(id.loc, vec![ty], builtin, vec![]));
    }

    // are we trying to resolve a function type?
    let function_first = if let ResolveTo::Type(resolve_to) = resolve_to {
        matches!(
            resolve_to,
            Type::InternalFunction { .. } | Type::ExternalFunction { .. }
        )
    } else {
        false
    };

    match ns.resolve_var(context.file_no, context.contract_no, id, function_first) {
        Some(Symbol::Variable(_, Some(var_contract_no), var_no)) => {
            let var_contract_no = *var_contract_no;
            let var_no = *var_no;

            let var = &ns.contracts[var_contract_no].variables[var_no];

            if var.constant {
                Ok(Expression::ConstantVariable(
                    id.loc,
                    var.ty.clone(),
                    Some(var_contract_no),
                    var_no,
                ))
            } else if context.constant {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!(
                        "cannot read contract variable ‘{}’ in constant expression",
                        id.name
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::StorageVariable(
                    id.loc,
                    Type::StorageRef(var.immutable, Box::new(var.ty.clone())),
                    var_contract_no,
                    var_no,
                ))
            }
        }
        Some(Symbol::Variable(_, None, var_no)) => {
            let var_no = *var_no;

            let var = &ns.constants[var_no];

            Ok(Expression::ConstantVariable(
                id.loc,
                var.ty.clone(),
                None,
                var_no,
            ))
        }
        Some(Symbol::Function(_)) => {
            let mut name_matches = 0;
            let mut expr = None;

            for function_no in
                available_functions(&id.name, true, context.file_no, context.contract_no, ns)
            {
                let func = &ns.functions[function_no];

                if func.ty != pt::FunctionTy::Function {
                    continue;
                }

                let ty = Type::InternalFunction {
                    params: func.params.iter().map(|p| p.ty.clone()).collect(),
                    mutability: func.mutability.clone(),
                    returns: func.returns.iter().map(|p| p.ty.clone()).collect(),
                };

                name_matches += 1;
                expr = Some(Expression::InternalFunction {
                    loc: id.loc,
                    ty,
                    function_no,
                    signature: if func.is_virtual || func.is_override.is_some() {
                        Some(func.signature.clone())
                    } else {
                        None
                    },
                });
            }

            if name_matches == 1 {
                Ok(expr.unwrap())
            } else {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!("function ‘{}’ is overloaded", id.name),
                ));
                Err(())
            }
        }
        sym => {
            let error = Namespace::wrong_symbol(sym, id);
            diagnostics.push(error);
            Err(())
        }
    }
}

fn subtract(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        false,
        false,
        ns,
        diagnostics,
    )?;

    if ty.is_rational() {
        let expr = Expression::Subtract(*loc, ty, false, Box::new(left), Box::new(right));

        return match eval_const_rational(&expr, context.contract_no, ns) {
            Ok(_) => Ok(expr),
            Err(diag) => {
                diagnostics.push(diag);
                Err(())
            }
        };
    }

    Ok(Expression::Subtract(
        *loc,
        ty.clone(),
        context.unchecked,
        Box::new(cast(&l.loc(), left, &ty, true, ns, diagnostics)?),
        Box::new(cast(&r.loc(), right, &ty, true, ns, diagnostics)?),
    ))
}

fn bitwise_or(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        true,
        false,
        ns,
        diagnostics,
    )?;

    Ok(Expression::BitwiseOr(
        *loc,
        ty.clone(),
        Box::new(cast(&l.loc(), left, &ty, true, ns, diagnostics)?),
        Box::new(cast(&r.loc(), right, &ty, true, ns, diagnostics)?),
    ))
}

fn bitwise_and(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        true,
        false,
        ns,
        diagnostics,
    )?;

    Ok(Expression::BitwiseAnd(
        *loc,
        ty.clone(),
        Box::new(cast(&l.loc(), left, &ty, true, ns, diagnostics)?),
        Box::new(cast(&r.loc(), right, &ty, true, ns, diagnostics)?),
    ))
}

fn bitwise_xor(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        true,
        false,
        ns,
        diagnostics,
    )?;

    Ok(Expression::BitwiseXor(
        *loc,
        ty.clone(),
        Box::new(cast(&l.loc(), left, &ty, true, ns, diagnostics)?),
        Box::new(cast(&r.loc(), right, &ty, true, ns, diagnostics)?),
    ))
}

fn shift_left(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;

    check_var_usage_expression(ns, &left, &right, symtable);
    // left hand side may be bytes/int/uint
    // right hand size may be int/uint
    let _ = get_int_length(&left.ty(), &l.loc(), true, ns, diagnostics)?;
    let (right_length, _) = get_int_length(&right.ty(), &r.loc(), false, ns, diagnostics)?;

    let left_type = left.ty();

    Ok(Expression::ShiftLeft(
        *loc,
        left_type.clone(),
        Box::new(left),
        Box::new(cast_shift_arg(loc, right, right_length, &left_type, ns)),
    ))
}

fn shift_right(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    let left_type = left.ty();
    // left hand side may be bytes/int/uint
    // right hand size may be int/uint
    let _ = get_int_length(&left_type, &l.loc(), true, ns, diagnostics)?;
    let (right_length, _) = get_int_length(&right.ty(), &r.loc(), false, ns, diagnostics)?;

    Ok(Expression::ShiftRight(
        *loc,
        left_type.clone(),
        Box::new(left),
        Box::new(cast_shift_arg(loc, right, right_length, &left_type, ns)),
        left_type.is_signed_int(),
    ))
}

fn multiply(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        false,
        false,
        ns,
        diagnostics,
    )?;

    if ty.is_rational() {
        let expr = Expression::Multiply(*loc, ty, false, Box::new(left), Box::new(right));

        return match eval_const_rational(&expr, context.contract_no, ns) {
            Ok(_) => Ok(expr),
            Err(diag) => {
                diagnostics.push(diag);
                Err(())
            }
        };
    }

    // If we don't know what type the result is going to be, make any possible result fit.
    if resolve_to == ResolveTo::Unknown {
        let bits = std::cmp::min(256, ty.bits(ns) * 2);

        if ty.is_signed_int() {
            multiply(
                loc,
                l,
                r,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&Type::Int(bits)),
            )
        } else {
            multiply(
                loc,
                l,
                r,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&Type::Uint(bits)),
            )
        }
    } else {
        Ok(Expression::Multiply(
            *loc,
            ty.clone(),
            context.unchecked,
            Box::new(cast(&l.loc(), left, &ty, true, ns, diagnostics)?),
            Box::new(cast(&r.loc(), right, &ty, true, ns, diagnostics)?),
        ))
    }
}

fn divide(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        false,
        false,
        ns,
        diagnostics,
    )?;

    Ok(Expression::Divide(
        *loc,
        ty.clone(),
        Box::new(cast(&l.loc(), left, &ty, true, ns, diagnostics)?),
        Box::new(cast(&r.loc(), right, &ty, true, ns, diagnostics)?),
    ))
}

fn modulo(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    let ty = coerce_number(
        &left.ty(),
        &l.loc(),
        &right.ty(),
        &r.loc(),
        false,
        false,
        ns,
        diagnostics,
    )?;

    Ok(Expression::Modulo(
        *loc,
        ty.clone(),
        Box::new(cast(&l.loc(), left, &ty, true, ns, diagnostics)?),
        Box::new(cast(&r.loc(), right, &ty, true, ns, diagnostics)?),
    ))
}

fn power(
    loc: &pt::Loc,
    b: &pt::Expression,
    e: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let mut base = expression(b, context, ns, symtable, diagnostics, resolve_to)?;

    // If we don't know what type the result is going to be, assume
    // the result is 256 bits
    if resolve_to == ResolveTo::Unknown {
        if base.ty().is_signed_int() {
            base = expression(
                b,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&Type::Int(256)),
            )?;
        } else {
            base = expression(
                b,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&Type::Uint(256)),
            )?;
        };
    }

    let exp = expression(e, context, ns, symtable, diagnostics, resolve_to)?;

    check_var_usage_expression(ns, &base, &exp, symtable);

    let base_type = base.ty();
    let exp_type = exp.ty();

    // solc-0.5.13 does not allow either base or exp to be signed
    if base_type.is_signed_int() || exp_type.is_signed_int() {
        diagnostics.push(Diagnostic::error(
            *loc,
            "exponation (**) is not allowed with signed types".to_string(),
        ));
        return Err(());
    }

    let ty = coerce_number(
        &base_type,
        &b.loc(),
        &exp_type,
        &e.loc(),
        false,
        false,
        ns,
        diagnostics,
    )?;

    Ok(Expression::Power(
        *loc,
        ty.clone(),
        context.unchecked,
        Box::new(cast(&b.loc(), base, &ty, true, ns, diagnostics)?),
        Box::new(cast(&e.loc(), exp, &ty, true, ns, diagnostics)?),
    ))
}

/// Resolve an new contract expression with positional arguments
fn constructor(
    loc: &pt::Loc,
    no: usize,
    args: &[pt::Expression],
    call_args: CallArgs,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    // The current contract cannot be constructed with new. In order to create
    // the contract, we need the code hash of the contract. Part of that code
    // will be code we're emitted here. So we end up with a crypto puzzle.
    let context_contract_no = match context.contract_no {
        Some(n) if n == no => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "new cannot construct current contract ‘{}’",
                    ns.contracts[no].name
                ),
            ));
            return Err(());
        }
        Some(n) => n,
        None => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "new contract not allowed in this context".to_string(),
            ));
            return Err(());
        }
    };

    if !ns.contracts[no].is_concrete() {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "cannot construct ‘{}’ of type ‘{}’",
                ns.contracts[no].name, ns.contracts[no].ty
            ),
        ));

        return Err(());
    }

    // check for circular references
    if circular_reference(no, context_contract_no, ns) {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "circular reference creating contract ‘{}’",
                ns.contracts[no].name
            ),
        ));
        return Err(());
    }

    if !ns.contracts[context_contract_no].creates.contains(&no) {
        ns.contracts[context_contract_no].creates.push(no);
    }

    match match_constructor_to_args(loc, args, no, context, ns, symtable, diagnostics) {
        Ok((constructor_no, cast_args)) => Ok(Expression::Constructor {
            loc: *loc,
            contract_no: no,
            constructor_no,
            args: cast_args,
            value: call_args.value,
            gas: call_args.gas,
            salt: call_args.salt,
            space: call_args.space,
        }),
        Err(()) => Err(()),
    }
}

/// Try and find constructor for arguments
pub fn match_constructor_to_args(
    loc: &pt::Loc,
    args: &[pt::Expression],
    contract_no: usize,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(Option<usize>, Vec<Expression>), ()> {
    let marker = diagnostics.len();

    // constructor call
    let mut constructor_count = 0;

    for function_no in ns.contracts[contract_no].functions.clone() {
        if !ns.functions[function_no].is_constructor() {
            continue;
        }

        constructor_count += 1;

        // ideally we shouldn't be cloning, but expression() takes a mutable reference to the namespace
        let params = ns.functions[function_no].params.clone();

        if params.len() != args.len() {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "constructor expects {} arguments, {} provided",
                    params.len(),
                    args.len()
                ),
            ));
            continue;
        }

        let mut matches = true;
        let mut cast_args = Vec::new();

        // resolve arguments for this constructor
        for (i, arg) in args.iter().enumerate() {
            let arg = match expression(
                arg,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&params[i].ty),
            ) {
                Ok(v) => v,
                Err(()) => {
                    matches = false;
                    continue;
                }
            };

            match cast(
                &arg.loc(),
                arg.clone(),
                &ns.functions[function_no].params[i].ty.clone(),
                true,
                ns,
                diagnostics,
            ) {
                Ok(expr) => cast_args.push(expr),
                Err(()) => {
                    matches = false;
                    continue;
                }
            }
        }

        if matches {
            return Ok((Some(function_no), cast_args));
        }
    }

    if constructor_count == 0 && args.is_empty() {
        return Ok((None, Vec::new()));
    }

    if constructor_count != 1 {
        diagnostics.truncate(marker);
        diagnostics.push(Diagnostic::error(
            *loc,
            "cannot find overloaded constructor which matches signature".to_string(),
        ));
    }

    Err(())
}

/// check if from creates to, recursively
fn circular_reference(from: usize, to: usize, ns: &Namespace) -> bool {
    if ns.contracts[from].creates.contains(&to) {
        return true;
    }

    ns.contracts[from]
        .creates
        .iter()
        .any(|n| circular_reference(*n, to, ns))
}

/// Resolve an new contract expression with named arguments
pub fn constructor_named_args(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::NamedArgument],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let (ty, call_args, _) = collect_call_args(ty, diagnostics)?;

    let call_args = parse_call_args(&call_args, false, context, ns, symtable, diagnostics)?;

    let no = match ns.resolve_type(context.file_no, context.contract_no, false, ty, diagnostics)? {
        Type::Contract(n) => n,
        _ => {
            diagnostics.push(Diagnostic::error(*loc, "contract expected".to_string()));
            return Err(());
        }
    };

    // The current contract cannot be constructed with new. In order to create
    // the contract, we need the code hash of the contract. Part of that code
    // will be code we're emitted here. So we end up with a crypto puzzle.
    let context_contract_no = match context.contract_no {
        Some(n) if n == no => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "new cannot construct current contract ‘{}’",
                    ns.contracts[no].name
                ),
            ));
            return Err(());
        }
        Some(n) => n,
        None => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "new contract not allowed in this context".to_string(),
            ));
            return Err(());
        }
    };

    if !ns.contracts[no].is_concrete() {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "cannot construct ‘{}’ of type ‘{}’",
                ns.contracts[no].name, ns.contracts[no].ty
            ),
        ));

        return Err(());
    }

    // check for circular references
    if circular_reference(no, context_contract_no, ns) {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "circular reference creating contract ‘{}’",
                ns.contracts[no].name
            ),
        ));
        return Err(());
    }

    if !ns.contracts[context_contract_no].creates.contains(&no) {
        ns.contracts[context_contract_no].creates.push(no);
    }

    let mut arguments: BTreeMap<&String, &pt::Expression> = BTreeMap::new();

    for arg in args {
        if let Some(prev) = arguments.get(&arg.name.name) {
            diagnostics.push(Diagnostic::error_with_note(
                *loc,
                format!("duplicate argument name ‘{}’", arg.name.name),
                prev.loc(),
                String::from("location of previous argument"),
            ));
            return Err(());
        }
        arguments.insert(&arg.name.name, &arg.expr);
    }

    let marker = diagnostics.len();
    let mut found_constructors = 0;

    // constructor call
    for function_no in ns.contracts[no].functions.clone() {
        if !ns.functions[function_no].is_constructor() {
            continue;
        }

        found_constructors += 1;

        let params_len = ns.functions[function_no].params.len();

        if params_len != args.len() {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "constructor expects {} arguments, {} provided",
                    params_len,
                    args.len()
                ),
            ));
            continue;
        }

        let mut matches = true;
        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for i in 0..params_len {
            let param = ns.functions[function_no].params[i].clone();
            let arg = match arguments.get(&param.name) {
                Some(a) => a,
                None => {
                    matches = false;
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!("missing argument ‘{}’ to constructor", param.name),
                    ));
                    break;
                }
            };

            let arg = match expression(
                arg,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&param.ty),
            ) {
                Ok(e) => e,
                Err(()) => {
                    matches = false;
                    break;
                }
            };

            match cast(&arg.loc(), arg, &param.ty, true, ns, diagnostics) {
                Ok(expr) => cast_args.push(expr),
                Err(()) => {
                    matches = false;
                    break;
                }
            }
        }

        if matches {
            return Ok(Expression::Constructor {
                loc: *loc,
                contract_no: no,
                constructor_no: Some(function_no),
                args: cast_args,
                value: call_args.value,
                gas: call_args.gas,
                salt: call_args.salt,
                space: call_args.space,
            });
        }
    }

    match found_constructors {
        0 => Ok(Expression::Constructor {
            loc: *loc,
            contract_no: no,
            constructor_no: None,
            args: Vec::new(),
            value: call_args.value,
            gas: call_args.gas,
            salt: call_args.salt,
            space: call_args.space,
        }),
        1 => Err(()),
        _ => {
            diagnostics.truncate(marker);
            diagnostics.push(Diagnostic::error(
                *loc,
                "cannot find overloaded constructor which matches signature".to_string(),
            ));

            Err(())
        }
    }
}

/// Resolve type(x).foo
pub fn type_name_expr(
    loc: &pt::Loc,
    args: &[pt::Expression],
    field: &pt::Identifier,
    context: &ExprContext,
    ns: &mut Namespace,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    if args.is_empty() {
        diagnostics.push(Diagnostic::error(
            *loc,
            "missing argument to type()".to_string(),
        ));
        return Err(());
    }

    if args.len() > 1 {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!("got {} arguments to type(), only one expected", args.len(),),
        ));
        return Err(());
    }

    let ty = ns.resolve_type(
        context.file_no,
        context.contract_no,
        false,
        &args[0],
        diagnostics,
    )?;

    match (&ty, field.name.as_str()) {
        (Type::Uint(_), "min") => {
            bigint_to_expression(loc, &BigInt::zero(), ns, diagnostics, resolve_to)
        }
        (Type::Uint(bits), "max") => {
            let max = BigInt::one().shl(*bits as usize).sub(1);
            bigint_to_expression(loc, &max, ns, diagnostics, resolve_to)
        }
        (Type::Int(bits), "min") => {
            let min = BigInt::zero().sub(BigInt::one().shl(*bits as usize - 1));
            bigint_to_expression(loc, &min, ns, diagnostics, resolve_to)
        }
        (Type::Int(bits), "max") => {
            let max = BigInt::one().shl(*bits as usize - 1).sub(1);
            bigint_to_expression(loc, &max, ns, diagnostics, resolve_to)
        }
        (Type::Contract(n), "name") => Ok(Expression::BytesLiteral(
            *loc,
            Type::String,
            ns.contracts[*n].name.as_bytes().to_vec(),
        )),
        (Type::Contract(n), "interfaceId") => {
            let contract = &ns.contracts[*n];

            if !contract.is_interface() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "type(…).interfaceId is permitted on interface, not {} {}",
                        contract.ty, contract.name
                    ),
                ));
                Err(())
            } else {
                Ok(Expression::InterfaceId(*loc, *n))
            }
        }
        (Type::Contract(no), "creationCode") | (Type::Contract(no), "runtimeCode") => {
            let contract_no = match context.contract_no {
                Some(contract_no) => contract_no,
                None => {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "type().{} not permitted outside of contract code",
                            field.name
                        ),
                    ));
                    return Err(());
                }
            };

            // check for circular references
            if *no == contract_no {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "containing our own contract code for ‘{}’ would generate infinite size contract",
                        ns.contracts[*no].name
                    ),
                ));
                return Err(());
            }

            if circular_reference(*no, contract_no, ns) {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "circular reference creating contract code for ‘{}’",
                        ns.contracts[*no].name
                    ),
                ));
                return Err(());
            }

            if !ns.contracts[contract_no].creates.contains(no) {
                ns.contracts[contract_no].creates.push(*no);
            }

            Ok(Expression::CodeLiteral(
                *loc,
                *no,
                field.name == "runtimeCode",
            ))
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "type ‘{}’ does not have type function {}",
                    ty.to_string(ns),
                    field.name
                ),
            ));
            Err(())
        }
    }
}

/// Resolve an new expression
pub fn new(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::Expression],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let (ty, call_args, call_args_loc) = collect_call_args(ty, diagnostics)?;

    let ty = ns.resolve_type(context.file_no, context.contract_no, false, ty, diagnostics)?;

    match &ty {
        Type::Array(ty, dim) => {
            if dim.last().unwrap().is_some() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "new cannot allocate fixed array type ‘{}’",
                        ty.to_string(ns)
                    ),
                ));
                return Err(());
            }

            if let Type::Contract(_) = ty.as_ref() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!("new cannot construct array of ‘{}’", ty.to_string(ns)),
                ));
                return Err(());
            }
        }
        Type::String | Type::DynamicBytes => {}
        Type::Contract(n) => {
            let call_args = parse_call_args(&call_args, false, context, ns, symtable, diagnostics)?;

            return constructor(loc, *n, args, call_args, context, ns, symtable, diagnostics);
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!("new cannot allocate type ‘{}’", ty.to_string(ns)),
            ));
            return Err(());
        }
    };

    if let Some(loc) = call_args_loc {
        diagnostics.push(Diagnostic::error(
            loc,
            "constructor arguments not permitted for allocation".to_string(),
        ));
        return Err(());
    }

    if args.len() != 1 {
        diagnostics.push(Diagnostic::error(
            *loc,
            "new dynamic array should have a single length argument".to_string(),
        ));
        return Err(());
    }
    let size_loc = args[0].loc();

    let size_expr = expression(
        &args[0],
        context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Type(&Type::Uint(32)),
    )?;

    used_variable(ns, &size_expr, symtable);

    let size = cast(&size_loc, size_expr, &Type::Uint(32), true, ns, diagnostics)?;

    Ok(Expression::AllocDynamicArray(
        *loc,
        ty,
        Box::new(size),
        None,
    ))
}

/// Test for equality; first check string equality, then integer equality
fn equal(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let left = expression(l, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
    let right = expression(r, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;

    check_var_usage_expression(ns, &left, &right, symtable);

    // Comparing stringliteral against stringliteral
    if let (Expression::BytesLiteral(_, _, l), Expression::BytesLiteral(_, _, r)) = (&left, &right)
    {
        return Ok(Expression::BoolLiteral(*loc, l == r));
    }

    let left_type = left.ty();
    let right_type = right.ty();

    // compare string against literal
    match (&left, &right_type.deref_any()) {
        (Expression::BytesLiteral(_, _, l), Type::String)
        | (Expression::BytesLiteral(_, _, l), Type::DynamicBytes) => {
            return Ok(Expression::StringCompare(
                *loc,
                StringLocation::RunTime(Box::new(cast(
                    &r.loc(),
                    right,
                    right_type.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?)),
                StringLocation::CompileTime(l.clone()),
            ));
        }
        _ => {}
    }

    match (&right, &left_type.deref_any()) {
        (Expression::BytesLiteral(_, _, literal), Type::String)
        | (Expression::BytesLiteral(_, _, literal), Type::DynamicBytes) => {
            return Ok(Expression::StringCompare(
                *loc,
                StringLocation::RunTime(Box::new(cast(
                    &l.loc(),
                    left,
                    left_type.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?)),
                StringLocation::CompileTime(literal.clone()),
            ));
        }
        _ => {}
    }

    // compare string
    match (&left_type.deref_any(), &right_type.deref_any()) {
        (Type::String, Type::String) | (Type::DynamicBytes, Type::DynamicBytes) => {
            return Ok(Expression::StringCompare(
                *loc,
                StringLocation::RunTime(Box::new(cast(
                    &l.loc(),
                    left,
                    left_type.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?)),
                StringLocation::RunTime(Box::new(cast(
                    &r.loc(),
                    right,
                    right_type.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?)),
            ));
        }
        _ => {}
    }

    let ty = coerce(&left_type, &l.loc(), &right_type, &r.loc(), ns, diagnostics)?;

    Ok(Expression::Equal(
        *loc,
        Box::new(cast(&l.loc(), left, &ty, true, ns, diagnostics)?),
        Box::new(cast(&r.loc(), right, &ty, true, ns, diagnostics)?),
    ))
}

/// Try string concatenation
fn addition(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let mut left = expression(l, context, ns, symtable, diagnostics, resolve_to)?;
    let mut right = expression(r, context, ns, symtable, diagnostics, resolve_to)?;
    check_var_usage_expression(ns, &left, &right, symtable);

    // Concatenate stringliteral with stringliteral
    if let (Expression::BytesLiteral(_, _, l), Expression::BytesLiteral(_, _, r)) = (&left, &right)
    {
        let mut c = Vec::with_capacity(l.len() + r.len());
        c.extend_from_slice(l);
        c.extend_from_slice(r);
        let length = c.len();
        return Ok(Expression::BytesLiteral(*loc, Type::Bytes(length as u8), c));
    }

    let left_type = left.ty();
    let right_type = right.ty();

    // compare string against literal
    match (&left, &right_type) {
        (Expression::BytesLiteral(_, _, l), Type::String)
        | (Expression::BytesLiteral(_, _, l), Type::DynamicBytes) => {
            return Ok(Expression::StringConcat(
                *loc,
                right_type,
                StringLocation::CompileTime(l.clone()),
                StringLocation::RunTime(Box::new(right)),
            ));
        }
        _ => {}
    }

    match (&right, &left_type) {
        (Expression::BytesLiteral(_, _, l), Type::String)
        | (Expression::BytesLiteral(_, _, l), Type::DynamicBytes) => {
            return Ok(Expression::StringConcat(
                *loc,
                left_type,
                StringLocation::RunTime(Box::new(left)),
                StringLocation::CompileTime(l.clone()),
            ));
        }
        _ => {}
    }

    // compare string
    match (&left_type, &right_type) {
        (Type::String, Type::String) | (Type::DynamicBytes, Type::DynamicBytes) => {
            return Ok(Expression::StringConcat(
                *loc,
                right_type,
                StringLocation::RunTime(Box::new(left)),
                StringLocation::RunTime(Box::new(right)),
            ));
        }
        _ => {}
    }

    let ty = coerce_number(
        &left_type,
        &l.loc(),
        &right_type,
        &r.loc(),
        false,
        false,
        ns,
        diagnostics,
    )?;

    // If we don't know what type the result is going to be
    if resolve_to == ResolveTo::Unknown {
        let bits = std::cmp::min(256, ty.bits(ns) * 2);
        let resolve_to = if ty.is_signed_int() {
            Type::Int(bits)
        } else {
            Type::Uint(bits)
        };

        left = expression(
            l,
            context,
            ns,
            symtable,
            diagnostics,
            ResolveTo::Type(&resolve_to),
        )?;
        right = expression(
            r,
            context,
            ns,
            symtable,
            diagnostics,
            ResolveTo::Type(&resolve_to),
        )?;
    }

    Ok(Expression::Add(
        *loc,
        ty.clone(),
        context.unchecked,
        Box::new(cast(&l.loc(), left, &ty, true, ns, diagnostics)?),
        Box::new(cast(&r.loc(), right, &ty, true, ns, diagnostics)?),
    ))
}

/// Resolve an assignment
pub fn assign_single(
    loc: &pt::Loc,
    left: &pt::Expression,
    right: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let mut context = context.clone();
    context.constant = false;

    let var = expression(
        left,
        &context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Unknown,
    )?;
    assigned_variable(ns, &var, symtable);

    let var_ty = var.ty();
    let val = expression(
        right,
        &context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Type(var_ty.deref_any()),
    )?;
    used_variable(ns, &val, symtable);
    match &var {
        Expression::ConstantVariable(loc, _, Some(contract_no), var_no) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "cannot assign to constant ‘{}’",
                    ns.contracts[*contract_no].variables[*var_no].name
                ),
            ));
            Err(())
        }
        Expression::ConstantVariable(loc, _, None, var_no) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!("cannot assign to constant ‘{}’", ns.constants[*var_no].name),
            ));
            Err(())
        }
        Expression::StorageVariable(loc, ty, var_contract_no, var_no) => {
            let store_var = &ns.contracts[*var_contract_no].variables[*var_no];

            if store_var.immutable {
                if let Some(function_no) = context.function_no {
                    if !ns.functions[function_no].is_constructor() {
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "cannot assign to immutable ‘{}’ outside of constructor",
                                store_var.name
                            ),
                        ));
                        return Err(());
                    }
                }
            }

            Ok(Expression::Assign(
                *loc,
                ty.clone(),
                Box::new(var.clone()),
                Box::new(cast(
                    &right.loc(),
                    val,
                    ty.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?),
            ))
        }
        Expression::Variable(_, var_ty, _) => Ok(Expression::Assign(
            *loc,
            var_ty.clone(),
            Box::new(var.clone()),
            Box::new(cast(&right.loc(), val, var_ty, true, ns, diagnostics)?),
        )),
        _ => match &var_ty {
            Type::Ref(r_ty) => Ok(Expression::Assign(
                *loc,
                var_ty.clone(),
                Box::new(var),
                Box::new(cast(&right.loc(), val, r_ty, true, ns, diagnostics)?),
            )),
            Type::StorageRef(immutable, r_ty) => {
                if *immutable {
                    if let Some(function_no) = context.function_no {
                        if !ns.functions[function_no].is_constructor() {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                "cannot assign to immutable outside of constructor".to_string(),
                            ));
                            return Err(());
                        }
                    }
                }

                Ok(Expression::Assign(
                    *loc,
                    var_ty.clone(),
                    Box::new(var),
                    Box::new(cast(&right.loc(), val, r_ty, true, ns, diagnostics)?),
                ))
            }
            _ => {
                diagnostics.push(Diagnostic::error(
                    var.loc(),
                    "expression is not assignable".to_string(),
                ));
                Err(())
            }
        },
    }
}

/// Resolve an assignment with an operator
fn assign_expr(
    loc: &pt::Loc,
    left: &pt::Expression,
    expr: &pt::Expression,
    right: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let mut context = context.clone();
    context.constant = false;

    let var = expression(
        left,
        &context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Unknown,
    )?;
    assigned_variable(ns, &var, symtable);
    let var_ty = var.ty();

    let resolve_to = if matches!(
        expr,
        pt::Expression::AssignShiftLeft(_, _, _) | pt::Expression::AssignShiftRight(_, _, _)
    ) {
        ResolveTo::Unknown
    } else {
        ResolveTo::Type(var_ty.deref_any())
    };

    let set = expression(right, &context, ns, symtable, diagnostics, resolve_to)?;
    used_variable(ns, &set, symtable);
    let set_type = set.ty();

    let op = |assign: Expression,
              ty: &Type,
              ns: &Namespace,
              diagnostics: &mut Vec<Diagnostic>|
     -> Result<Expression, ()> {
        let set = match expr {
            pt::Expression::AssignShiftLeft(_, _, _)
            | pt::Expression::AssignShiftRight(_, _, _) => {
                let left_length = get_int_length(ty, loc, true, ns, diagnostics)?;
                let right_length = get_int_length(&set_type, &left.loc(), false, ns, diagnostics)?;

                // TODO: does shifting by negative value need compiletime/runtime check?
                if left_length == right_length {
                    set
                } else if right_length < left_length && set_type.is_signed_int() {
                    Expression::SignExt(*loc, ty.clone(), Box::new(set))
                } else if right_length < left_length && !set_type.is_signed_int() {
                    Expression::ZeroExt(*loc, ty.clone(), Box::new(set))
                } else {
                    Expression::Trunc(*loc, ty.clone(), Box::new(set))
                }
            }
            _ => cast(&right.loc(), set, ty, true, ns, diagnostics)?,
        };

        Ok(match expr {
            pt::Expression::AssignAdd(_, _, _) => Expression::Add(
                *loc,
                ty.clone(),
                context.unchecked,
                Box::new(assign),
                Box::new(set),
            ),
            pt::Expression::AssignSubtract(_, _, _) => Expression::Subtract(
                *loc,
                ty.clone(),
                context.unchecked,
                Box::new(assign),
                Box::new(set),
            ),
            pt::Expression::AssignMultiply(_, _, _) => Expression::Multiply(
                *loc,
                ty.clone(),
                context.unchecked,
                Box::new(assign),
                Box::new(set),
            ),
            pt::Expression::AssignOr(_, _, _) => {
                Expression::BitwiseOr(*loc, ty.clone(), Box::new(assign), Box::new(set))
            }
            pt::Expression::AssignAnd(_, _, _) => {
                Expression::BitwiseAnd(*loc, ty.clone(), Box::new(assign), Box::new(set))
            }
            pt::Expression::AssignXor(_, _, _) => {
                Expression::BitwiseXor(*loc, ty.clone(), Box::new(assign), Box::new(set))
            }
            pt::Expression::AssignShiftLeft(_, _, _) => {
                Expression::ShiftLeft(*loc, ty.clone(), Box::new(assign), Box::new(set))
            }
            pt::Expression::AssignShiftRight(_, _, _) => Expression::ShiftRight(
                *loc,
                ty.clone(),
                Box::new(assign),
                Box::new(set),
                ty.is_signed_int(),
            ),
            pt::Expression::AssignDivide(_, _, _) => {
                Expression::Divide(*loc, ty.clone(), Box::new(assign), Box::new(set))
            }
            pt::Expression::AssignModulo(_, _, _) => {
                Expression::Modulo(*loc, ty.clone(), Box::new(assign), Box::new(set))
            }
            _ => unreachable!(),
        })
    };

    match &var {
        Expression::ConstantVariable(loc, _, Some(contract_no), var_no) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "cannot assign to constant ‘{}’",
                    ns.contracts[*contract_no].variables[*var_no].name
                ),
            ));
            Err(())
        }
        Expression::ConstantVariable(loc, _, None, var_no) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!("cannot assign to constant ‘{}’", ns.constants[*var_no].name),
            ));
            Err(())
        }
        Expression::Variable(_, _, n) => {
            match var_ty {
                Type::Bytes(_) | Type::Int(_) | Type::Uint(_) => (),
                _ => {
                    diagnostics.push(Diagnostic::error(
                        var.loc(),
                        format!(
                            "variable ‘{}’ of incorrect type {}",
                            symtable.get_name(*n),
                            var_ty.to_string(ns)
                        ),
                    ));
                    return Err(());
                }
            };
            Ok(Expression::Assign(
                *loc,
                Type::Void,
                Box::new(var.clone()),
                Box::new(op(var, &var_ty, ns, diagnostics)?),
            ))
        }
        _ => match &var_ty {
            Type::Ref(r_ty) => match r_ty.as_ref() {
                Type::Bytes(_) | Type::Int(_) | Type::Uint(_) => Ok(Expression::Assign(
                    *loc,
                    Type::Void,
                    Box::new(var.clone()),
                    Box::new(op(
                        cast(loc, var, r_ty, true, ns, diagnostics)?,
                        r_ty,
                        ns,
                        diagnostics,
                    )?),
                )),
                _ => {
                    diagnostics.push(Diagnostic::error(
                        var.loc(),
                        format!("assigning to incorrect type {}", r_ty.to_string(ns)),
                    ));
                    Err(())
                }
            },
            Type::StorageRef(immutable, r_ty) => {
                if *immutable {
                    if let Some(function_no) = context.function_no {
                        if !ns.functions[function_no].is_constructor() {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                "cannot assign to immutable outside of constructor".to_string(),
                            ));
                            return Err(());
                        }
                    }
                }

                match r_ty.as_ref() {
                    Type::Bytes(_) | Type::Int(_) | Type::Uint(_) => Ok(Expression::Assign(
                        *loc,
                        Type::Void,
                        Box::new(var.clone()),
                        Box::new(op(
                            cast(loc, var, r_ty, true, ns, diagnostics)?,
                            r_ty,
                            ns,
                            diagnostics,
                        )?),
                    )),
                    _ => {
                        diagnostics.push(Diagnostic::error(
                            var.loc(),
                            format!("assigning to incorrect type {}", r_ty.to_string(ns)),
                        ));
                        Err(())
                    }
                }
            }
            _ => {
                diagnostics.push(Diagnostic::error(
                    var.loc(),
                    "expression is not assignable".to_string(),
                ));
                Err(())
            }
        },
    }
}

/// Resolve an increment/decrement with an operator
fn incr_decr(
    v: &pt::Expression,
    expr: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let op = |e: Expression, ty: Type| -> Expression {
        match expr {
            pt::Expression::PreIncrement(loc, _) => {
                Expression::PreIncrement(*loc, ty, context.unchecked, Box::new(e))
            }
            pt::Expression::PreDecrement(loc, _) => {
                Expression::PreDecrement(*loc, ty, context.unchecked, Box::new(e))
            }
            pt::Expression::PostIncrement(loc, _) => {
                Expression::PostIncrement(*loc, ty, context.unchecked, Box::new(e))
            }
            pt::Expression::PostDecrement(loc, _) => {
                Expression::PostDecrement(*loc, ty, context.unchecked, Box::new(e))
            }
            _ => unreachable!(),
        }
    };

    let mut context = context.clone();

    context.constant = false;

    let var = expression(v, &context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
    used_variable(ns, &var, symtable);
    let var_ty = var.ty();

    match &var {
        Expression::ConstantVariable(loc, _, Some(contract_no), var_no) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "cannot assign to constant ‘{}’",
                    ns.contracts[*contract_no].variables[*var_no].name
                ),
            ));
            Err(())
        }
        Expression::ConstantVariable(loc, _, None, var_no) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!("cannot assign to constant ‘{}’", ns.constants[*var_no].name),
            ));
            Err(())
        }
        Expression::Variable(_, ty, n) => {
            match ty {
                Type::Int(_) | Type::Uint(_) => (),
                _ => {
                    diagnostics.push(Diagnostic::error(
                        var.loc(),
                        format!(
                            "variable ‘{}’ of incorrect type {}",
                            symtable.get_name(*n),
                            var_ty.to_string(ns)
                        ),
                    ));
                    return Err(());
                }
            };
            Ok(op(var.clone(), ty.clone()))
        }
        _ => match &var_ty {
            Type::Ref(r_ty) => match r_ty.as_ref() {
                Type::Int(_) | Type::Uint(_) => Ok(op(var, r_ty.as_ref().clone())),
                _ => {
                    diagnostics.push(Diagnostic::error(
                        var.loc(),
                        format!("assigning to incorrect type {}", r_ty.to_string(ns)),
                    ));
                    Err(())
                }
            },
            Type::StorageRef(immutable, r_ty) => {
                if *immutable {
                    if let Some(function_no) = context.function_no {
                        if !ns.functions[function_no].is_constructor() {
                            diagnostics.push(Diagnostic::error(
                                var.loc(),
                                "cannot assign to immutable outside of constructor".to_string(),
                            ));
                            return Err(());
                        }
                    }
                }
                match r_ty.as_ref() {
                    Type::Int(_) | Type::Uint(_) => Ok(op(var, r_ty.as_ref().clone())),
                    _ => {
                        diagnostics.push(Diagnostic::error(
                            var.loc(),
                            format!("assigning to incorrect type {}", r_ty.to_string(ns)),
                        ));
                        Err(())
                    }
                }
            }
            _ => {
                diagnostics.push(Diagnostic::error(
                    var.loc(),
                    "expression is not modifiable".to_string(),
                ));
                Err(())
            }
        },
    }
}

/// Try to resolve expression as an enum value. An enum can be prefixed
/// with import symbols, contract namespace before the enum type
fn enum_value(
    loc: &pt::Loc,
    expr: &pt::Expression,
    id: &pt::Identifier,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &Namespace,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Option<Expression>, ()> {
    let mut namespace = Vec::new();

    let mut expr = expr;

    // the first element of the path is the deepest in the parse tree,
    // so walk down and add to a list
    while let pt::Expression::MemberAccess(_, member, name) = expr {
        namespace.push(name);

        expr = member.as_ref();
    }

    if let pt::Expression::Variable(name) = expr {
        namespace.push(name);
    } else {
        return Ok(None);
    }

    // The leading part of the namespace can be import variables
    let mut file_no = file_no;

    // last element in our namespace vector is first element
    while let Some(name) = namespace.last().map(|f| f.name.clone()) {
        if let Some(Symbol::Import(_, import_file_no)) =
            ns.variable_symbols.get(&(file_no, None, name))
        {
            file_no = *import_file_no;
            namespace.pop();
        } else {
            break;
        }
    }

    if namespace.is_empty() {
        return Ok(None);
    }

    let mut contract_no = contract_no;

    if let Some(no) = ns.resolve_contract(file_no, namespace.last().unwrap()) {
        contract_no = Some(no);
        namespace.pop();
    }

    if namespace.len() != 1 {
        return Ok(None);
    }

    if let Some(e) = ns.resolve_enum(file_no, contract_no, namespace[0]) {
        match ns.enums[e].values.get(&id.name) {
            Some((_, val)) => Ok(Some(Expression::NumberLiteral(
                *loc,
                Type::Enum(e),
                BigInt::from_usize(*val).unwrap(),
            ))),
            None => {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!("enum {} does not have value {}", ns.enums[e], id.name),
                ));
                Err(())
            }
        }
    } else {
        Ok(None)
    }
}

/// Resolve an array subscript expression
fn member_access(
    loc: &pt::Loc,
    e: &pt::Expression,
    id: &pt::Identifier,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    // is it a builtin special variable like "block.timestamp"
    if let pt::Expression::Variable(namespace) = e {
        if let Some((builtin, ty)) =
            builtin::builtin_var(loc, Some(&namespace.name), &id.name, ns, diagnostics)
        {
            return Ok(Expression::Builtin(*loc, vec![ty], builtin, vec![]));
        }

        if builtin::builtin_namespace(&namespace.name) {
            diagnostics.push(Diagnostic::error(
                e.loc(),
                format!("builtin ‘{}.{}’ does not exist", namespace.name, id.name),
            ));

            return Err(());
        }
    }

    // is it an enum value
    if let Some(expr) = enum_value(
        loc,
        e,
        id,
        context.file_no,
        context.contract_no,
        ns,
        diagnostics,
    )? {
        return Ok(expr);
    }

    // is it a constant (unless basecontract is a local variable)
    if let Some(expr) = contract_constant(
        e,
        id,
        context.file_no,
        ns,
        symtable,
        diagnostics,
        resolve_to,
    )? {
        return Ok(expr);
    }

    // is it a basecontract.function.selector expression (unless basecontract is a local variable)
    if let pt::Expression::Variable(namespace) = e {
        if symtable.find(&namespace.name).is_none() {
            if let Some(call_contract_no) = ns.resolve_contract(context.file_no, namespace) {
                // find function with this name
                let mut name_matches = 0;
                let mut expr = Err(());

                for function_no in &ns.contracts[call_contract_no].functions {
                    let func = &ns.functions[*function_no];

                    if func.name != id.name || func.ty != pt::FunctionTy::Function {
                        continue;
                    }

                    name_matches += 1;

                    expr = Ok(Expression::InternalFunction {
                        loc: e.loc(),
                        ty: function_type(func, false),
                        function_no: *function_no,
                        signature: None,
                    })
                }

                return match name_matches {
                    0 => {
                        diagnostics.push(Diagnostic::error(
                            e.loc(),
                            format!(
                                "contract ‘{}’ does not have a function called ‘{}’",
                                ns.contracts[call_contract_no].name, id.name,
                            ),
                        ));
                        Err(())
                    }
                    1 => expr,
                    _ => {
                        diagnostics.push(Diagnostic::error(
                            e.loc(),
                            format!(
                                "function ‘{}’ of contract ‘{}’ is overloaded",
                                id.name, ns.contracts[call_contract_no].name,
                            ),
                        ));
                        Err(())
                    }
                };
            }
        }
    }

    // is of the form "type(x).field", like type(c).min
    if let pt::Expression::FunctionCall(_, name, args) = e {
        if let pt::Expression::Variable(func_name) = name.as_ref() {
            if func_name.name == "type" {
                return type_name_expr(loc, args, id, context, ns, diagnostics, resolve_to);
            }
        }
    }

    let expr = expression(e, context, ns, symtable, diagnostics, resolve_to)?;
    let expr_ty = expr.ty();

    // Dereference if need to. This could be struct-in-struct for
    // example.
    let (expr, expr_ty) = if let Type::Ref(ty) = &expr_ty {
        (
            Expression::Load(*loc, expr_ty.clone(), Box::new(expr)),
            ty.as_ref().clone(),
        )
    } else {
        (expr, expr_ty)
    };

    match expr_ty {
        Type::Bytes(n) => {
            if id.name == "length" {
                //We should not eliminate an array from the code when 'length' is called
                //So the variable is also assigned a value to be read from 'length'
                assigned_variable(ns, &expr, symtable);
                used_variable(ns, &expr, symtable);
                return Ok(Expression::NumberLiteral(
                    *loc,
                    Type::Uint(8),
                    BigInt::from_u8(n).unwrap(),
                ));
            }
        }
        Type::Array(_, dim) => {
            if id.name == "length" {
                return match dim.last().unwrap() {
                    None => Ok(Expression::DynamicArrayLength(*loc, Box::new(expr))),
                    Some(d) => {
                        //We should not eliminate an array from the code when 'length' is called
                        //So the variable is also assigned a value to be read from 'length'
                        assigned_variable(ns, &expr, symtable);
                        used_variable(ns, &expr, symtable);
                        bigint_to_expression(
                            loc,
                            d,
                            ns,
                            diagnostics,
                            ResolveTo::Type(&Type::Uint(32)),
                        )
                    }
                };
            }
        }
        Type::String | Type::DynamicBytes => {
            if id.name == "length" {
                return Ok(Expression::DynamicArrayLength(*loc, Box::new(expr)));
            }
        }
        Type::StorageRef(immutable, r) => match *r {
            Type::Struct(n) => {
                return if let Some((field_no, field)) = ns.structs[n]
                    .fields
                    .iter()
                    .enumerate()
                    .find(|(_, field)| id.name == field.name)
                {
                    Ok(Expression::StructMember(
                        id.loc,
                        Type::StorageRef(immutable, Box::new(field.ty.clone())),
                        Box::new(expr),
                        field_no,
                    ))
                } else {
                    diagnostics.push(Diagnostic::error(
                        id.loc,
                        format!(
                            "struct ‘{}’ does not have a field called ‘{}’",
                            ns.structs[n].name, id.name
                        ),
                    ));
                    Err(())
                }
            }
            Type::Array(_, dim) => {
                if id.name == "length" {
                    let elem_ty = expr.ty().storage_array_elem().deref_into();

                    if let Some(dim) = &dim[0] {
                        // sparse array could be large than ns.storage_type() on Solana
                        if dim.bits() > ns.storage_type().bits(ns) as u64 {
                            return Ok(Expression::StorageArrayLength {
                                loc: id.loc,
                                ty: Type::Uint(256),
                                array: Box::new(expr),
                                elem_ty,
                            });
                        }
                    }

                    return Ok(Expression::StorageArrayLength {
                        loc: id.loc,
                        ty: ns.storage_type(),
                        array: Box::new(expr),
                        elem_ty,
                    });
                }
            }
            Type::Bytes(_) | Type::DynamicBytes => {
                if id.name == "length" {
                    let elem_ty = expr.ty().storage_array_elem().deref_into();

                    return Ok(Expression::StorageArrayLength {
                        loc: id.loc,
                        ty: Type::Uint(32),
                        array: Box::new(expr),
                        elem_ty,
                    });
                }
            }
            _ => {}
        },
        Type::Struct(n) => {
            if let Some((i, f)) = ns.structs[n]
                .fields
                .iter()
                .enumerate()
                .find(|f| id.name == f.1.name)
            {
                return Ok(Expression::StructMember(
                    id.loc,
                    Type::Ref(Box::new(f.ty.clone())),
                    Box::new(expr),
                    i,
                ));
            } else {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!(
                        "struct ‘{}’ does not have a field called ‘{}’",
                        ns.structs[n], id.name
                    ),
                ));
                return Err(());
            }
        }
        Type::Address(_) => {
            if id.name == "balance" {
                if ns.target.is_substrate() {
                    let mut is_this = false;

                    if let Expression::Cast(_, _, this) = &expr {
                        if let Expression::Builtin(_, _, Builtin::GetAddress, _) = this.as_ref() {
                            is_this = true;
                        }
                    }

                    if !is_this {
                        diagnostics.push(Diagnostic::error(
                            expr.loc(),
                            "substrate can only retrieve balance of this, like ‘address(this).balance’".to_string(),
                        ));
                        return Err(());
                    }
                }
                used_variable(ns, &expr, symtable);
                return Ok(Expression::Builtin(
                    *loc,
                    vec![Type::Value],
                    Builtin::Balance,
                    vec![expr],
                ));
            }
        }
        Type::Contract(ref_contract_no) => {
            let mut name_matches = 0;
            let mut ext_expr = Err(());

            for function_no in &ns.contracts[ref_contract_no].functions {
                let func = &ns.functions[*function_no];

                if func.name != id.name || func.ty != pt::FunctionTy::Function || !func.is_public()
                {
                    continue;
                }

                let ty = Type::ExternalFunction {
                    params: func.params.iter().map(|p| p.ty.clone()).collect(),
                    mutability: func.mutability.clone(),
                    returns: func.returns.iter().map(|p| p.ty.clone()).collect(),
                };

                name_matches += 1;
                ext_expr = Ok(Expression::ExternalFunction {
                    loc: id.loc,
                    ty,
                    address: Box::new(expr.clone()),
                    function_no: *function_no,
                });
            }

            #[allow(clippy::comparison_chain)]
            return if name_matches == 0 {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!(
                        "{} ‘{}’ has no public function ‘{}’",
                        ns.contracts[ref_contract_no].ty,
                        ns.contracts[ref_contract_no].name,
                        id.name
                    ),
                ));
                Err(())
            } else if name_matches == 1 {
                ext_expr
            } else {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!(
                        "function ‘{}’ of {} ‘{}’ is overloaded",
                        id.name,
                        ns.contracts[ref_contract_no].ty,
                        ns.contracts[ref_contract_no].name
                    ),
                ));
                Err(())
            };
        }
        Type::ExternalFunction { .. } => {
            if id.name == "address" {
                used_variable(ns, &expr, symtable);
                return Ok(Expression::Builtin(
                    e.loc(),
                    vec![Type::Address(false)],
                    Builtin::ExternalFunctionAddress,
                    vec![expr],
                ));
            }
            if id.name == "selector" {
                used_variable(ns, &expr, symtable);
                return Ok(Expression::Builtin(
                    e.loc(),
                    vec![Type::Bytes(4)],
                    Builtin::FunctionSelector,
                    vec![expr],
                ));
            }
        }
        Type::InternalFunction { .. } => {
            if let Expression::InternalFunction { .. } = expr {
                if id.name == "selector" {
                    used_variable(ns, &expr, symtable);
                    return Ok(Expression::Builtin(
                        e.loc(),
                        vec![Type::Bytes(4)],
                        Builtin::FunctionSelector,
                        vec![expr],
                    ));
                }
            }
        }
        _ => (),
    }

    diagnostics.push(Diagnostic::error(*loc, format!("‘{}’ not found", id.name)));

    Err(())
}

fn contract_constant(
    e: &pt::Expression,
    id: &pt::Identifier,
    file_no: usize,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Option<Expression>, ()> {
    let namespace = match e {
        pt::Expression::Variable(namespace) => namespace,
        _ => return Ok(None),
    };

    if symtable.find(&namespace.name).is_some() {
        return Ok(None);
    }

    if let Some(contract_no) = ns.resolve_contract(file_no, namespace) {
        if let Some((var_no, var)) = ns.contracts[contract_no]
            .variables
            .iter_mut()
            .enumerate()
            .find(|(_, variable)| variable.name == id.name)
        {
            if !var.constant {
                let resolve_function = if let ResolveTo::Type(ty) = resolve_to {
                    matches!(
                        ty,
                        Type::InternalFunction { .. } | Type::ExternalFunction { .. }
                    )
                } else {
                    false
                };

                if resolve_function {
                    // requested function, fall through
                    return Ok(None);
                } else {
                    diagnostics.push(Diagnostic::error(
                        e.loc(),
                        format!(
                            "need instance of contract ‘{}’ to get variable value ‘{}’",
                            ns.contracts[contract_no].name,
                            ns.contracts[contract_no].variables[var_no].name,
                        ),
                    ));
                    return Err(());
                }
            }

            var.read = true;

            return Ok(Some(Expression::ConstantVariable(
                var.loc,
                var.ty.clone(),
                Some(contract_no),
                var_no,
            )));
        }
    }

    Ok(None)
}

/// Resolve an array subscript expression
fn array_subscript(
    loc: &pt::Loc,
    array: &pt::Expression,
    index: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let array = expression(
        array,
        context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Unknown,
    )?;
    let array_ty = array.ty();

    if array.ty().is_mapping() {
        return mapping_subscript(loc, array, index, context, ns, symtable, diagnostics);
    }

    let index_width_ty = if array_ty.is_contract_storage() && !array_ty.is_storage_bytes() {
        Type::Uint(256)
    } else {
        Type::Uint(32)
    };

    let mut index = expression(
        index,
        context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Type(&index_width_ty),
    )?;

    let index_ty = index.ty();

    match index_ty.deref_any() {
        Type::Uint(_) => (),
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "array subscript must be an unsigned integer, not ‘{}’",
                    index.ty().to_string(ns)
                ),
            ));
            return Err(());
        }
    };

    if array_ty.is_storage_bytes() {
        return Ok(Expression::StorageBytesSubscript(
            *loc,
            Box::new(array),
            Box::new(cast(
                &index.loc(),
                index,
                &Type::Uint(32),
                false,
                ns,
                diagnostics,
            )?),
        ));
    }

    if index_ty.is_contract_storage() {
        // make sure we load the index value from storage
        index = cast(
            &index.loc(),
            index,
            index_ty.deref_any(),
            true,
            ns,
            diagnostics,
        )?;
    }

    match array_ty.deref_any() {
        Type::Bytes(_) | Type::Array(_, _) | Type::DynamicBytes => {
            if array_ty.is_contract_storage() {
                Ok(Expression::Subscript(
                    *loc,
                    array_ty,
                    Box::new(array),
                    Box::new(index),
                ))
            } else {
                let array = cast(
                    &array.loc(),
                    array,
                    array_ty.deref_any(),
                    true,
                    ns,
                    diagnostics,
                )?;

                Ok(Expression::Subscript(
                    *loc,
                    array_ty,
                    Box::new(array),
                    Box::new(index),
                ))
            }
        }
        Type::String => {
            diagnostics.push(Diagnostic::error(
                array.loc(),
                "array subscript is not permitted on string".to_string(),
            ));
            Err(())
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                array.loc(),
                "expression is not an array".to_string(),
            ));
            Err(())
        }
    }
}

/// Resolve a function call with positional arguments
fn struct_literal(
    loc: &pt::Loc,
    struct_no: usize,
    args: &[pt::Expression],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let struct_def = ns.structs[struct_no].clone();

    if args.len() != struct_def.fields.len() {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "struct ‘{}’ has {} fields, not {}",
                struct_def.name,
                struct_def.fields.len(),
                args.len()
            ),
        ));
        Err(())
    } else {
        let mut fields = Vec::new();

        for (i, a) in args.iter().enumerate() {
            let expr = expression(
                a,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&struct_def.fields[i].ty),
            )?;
            used_variable(ns, &expr, symtable);
            fields.push(cast(
                loc,
                expr,
                &struct_def.fields[i].ty,
                true,
                ns,
                diagnostics,
            )?);
        }

        let ty = Type::Struct(struct_no);

        Ok(Expression::StructLiteral(*loc, ty, fields))
    }
}

/// Resolve a function call via function type
/// Function types do not have names so call cannot be using named parameters
fn call_function_type(
    loc: &pt::Loc,
    expr: &pt::Expression,
    args: &[pt::Expression],
    call_args: &[&pt::NamedArgument],
    call_args_loc: Option<pt::Loc>,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let mut function = expression(expr, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;

    let mut ty = function.ty();

    match ty {
        Type::StorageRef(_, real_ty) | Type::Ref(real_ty) => {
            ty = *real_ty;
            function = cast(&expr.loc(), function, &ty, true, ns, diagnostics)?;
        }
        _ => (),
    };

    if let Type::InternalFunction {
        params, returns, ..
    } = ty
    {
        if let Some(loc) = call_args_loc {
            diagnostics.push(Diagnostic::error(
                loc,
                "call arguments not permitted for internal calls".to_string(),
            ));
        }

        if params.len() != args.len() {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "function expects {} arguments, {} provided",
                    params.len(),
                    args.len()
                ),
            ));
            return Err(());
        }

        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for (i, arg) in args.iter().enumerate() {
            let arg = expression(
                arg,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&params[i]),
            )?;

            cast_args.push(cast(
                &arg.loc(),
                arg.clone(),
                &params[i],
                true,
                ns,
                diagnostics,
            )?);
        }

        Ok(Expression::InternalFunctionCall {
            loc: *loc,
            returns: if returns.is_empty() {
                vec![Type::Void]
            } else {
                returns
            },
            function: Box::new(function),
            args: cast_args,
        })
    } else if let Type::ExternalFunction {
        returns,
        params,
        mutability,
    } = ty
    {
        let call_args = parse_call_args(call_args, true, context, ns, symtable, diagnostics)?;

        let value = if let Some(value) = call_args.value {
            if !value.const_zero(context.contract_no, ns)
                && !matches!(mutability, Mutability::Payable(_))
            {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "sending value to function type ‘{}’ which is not payable",
                        function.ty().to_string(ns),
                    ),
                ));
                return Err(());
            }

            Some(value)
        } else {
            None
        };

        if params.len() != args.len() {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "function expects {} arguments, {} provided",
                    params.len(),
                    args.len()
                ),
            ));
            return Err(());
        }

        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for (i, arg) in args.iter().enumerate() {
            let arg = expression(
                arg,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&params[i]),
            )?;

            cast_args.push(cast(
                &arg.loc(),
                arg.clone(),
                &params[i],
                true,
                ns,
                diagnostics,
            )?);
        }

        Ok(Expression::ExternalFunctionCall {
            loc: *loc,
            returns: if returns.is_empty() {
                vec![Type::Void]
            } else {
                returns
            },
            function: Box::new(function),
            args: cast_args,
            gas: call_args.gas,
            value,
        })
    } else {
        diagnostics.push(Diagnostic::error(
            *loc,
            "expression is not a function".to_string(),
        ));
        Err(())
    }
}

/// Create a list of functions that can be called in this context. If global is true, then
/// include functions outside of contracts
pub fn available_functions(
    name: &str,
    global: bool,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &Namespace,
) -> Vec<usize> {
    let mut list = Vec::new();

    if global {
        if let Some(Symbol::Function(v)) =
            ns.function_symbols.get(&(file_no, None, name.to_owned()))
        {
            list.extend(v.iter().map(|(_, func_no)| *func_no));
        }
    }

    if let Some(contract_no) = contract_no {
        list.extend(
            ns.contracts[contract_no]
                .all_functions
                .keys()
                .filter_map(|func_no| {
                    if ns.functions[*func_no].name == name && ns.functions[*func_no].has_body {
                        Some(*func_no)
                    } else {
                        None
                    }
                }),
        );
    }

    list
}

/// Create a list of functions that can be called via super
pub fn available_super_functions(name: &str, contract_no: usize, ns: &Namespace) -> Vec<usize> {
    let mut list = Vec::new();

    for base_contract_no in visit_bases(contract_no, ns).into_iter().rev() {
        if base_contract_no == contract_no {
            continue;
        }

        list.extend(
            ns.contracts[base_contract_no]
                .all_functions
                .keys()
                .filter_map(|func_no| {
                    if ns.functions[*func_no].name == name {
                        Some(*func_no)
                    } else {
                        None
                    }
                }),
        );
    }

    list
}

/// Resolve a function call with positional arguments
pub fn call_position_args(
    loc: &pt::Loc,
    id: &pt::Identifier,
    func_ty: pt::FunctionTy,
    args: &[pt::Expression],
    function_nos: Vec<usize>,
    virtual_call: bool,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let mut name_matches = 0;
    let mut errors = Vec::new();

    // Try to resolve as a function call
    for function_no in function_nos {
        let func = &ns.functions[function_no];

        if func.ty != func_ty {
            continue;
        }

        name_matches += 1;

        let params_len = func.params.len();

        if params_len != args.len() {
            errors.push(Diagnostic::error(
                *loc,
                format!(
                    "{} expects {} arguments, {} provided",
                    func.ty,
                    params_len,
                    args.len()
                ),
            ));
            continue;
        }

        let mut matches = true;
        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for (i, arg) in args.iter().enumerate() {
            let ty = ns.functions[function_no].params[i].ty.clone();

            let arg = match expression(
                arg,
                context,
                ns,
                symtable,
                &mut errors,
                ResolveTo::Type(&ty),
            ) {
                Ok(e) => e,
                Err(_) => {
                    matches = false;
                    continue;
                }
            };

            match cast(&arg.loc(), arg.clone(), &ty, true, ns, &mut errors) {
                Ok(expr) => cast_args.push(expr),
                Err(_) => {
                    matches = false;
                    continue;
                }
            }
        }

        if !matches {
            continue;
        }

        let func = &ns.functions[function_no];

        if func.contract_no != context.contract_no && func.is_private() {
            errors.push(Diagnostic::error_with_note(
                *loc,
                format!("cannot call private {}", func.ty),
                func.loc,
                format!("declaration of {} ‘{}’", func.ty, func.name),
            ));

            continue;
        }

        let returns = function_returns(func);
        let ty = function_type(func, false);

        return Ok(Expression::InternalFunctionCall {
            loc: *loc,
            returns,
            function: Box::new(Expression::InternalFunction {
                loc: *loc,
                ty,
                function_no,
                signature: if virtual_call && (func.is_virtual || func.is_override.is_some()) {
                    Some(func.signature.clone())
                } else {
                    None
                },
            }),
            args: cast_args,
        });
    }

    match name_matches {
        0 => {
            if func_ty == pt::FunctionTy::Modifier {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!("unknown modifier ‘{}’", id.name),
                ));
            } else {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!("unknown {} or type ‘{}’", func_ty, id.name),
                ));
            }
        }
        1 => diagnostics.extend(errors),
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!("cannot find overloaded {} which matches signature", func_ty),
            ));
        }
    }

    Err(())
}

/// Resolve a function call with named arguments
fn function_call_with_named_args(
    loc: &pt::Loc,
    id: &pt::Identifier,
    args: &[pt::NamedArgument],
    function_nos: Vec<usize>,
    virtual_call: bool,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let mut arguments = HashMap::new();

    for arg in args {
        if arguments.contains_key(&arg.name.name) {
            diagnostics.push(Diagnostic::error(
                arg.name.loc,
                format!("duplicate argument with name ‘{}’", arg.name.name),
            ));
            return Err(());
        }

        arguments.insert(&arg.name.name, &arg.expr);
    }

    // Try to resolve as a function call
    let mut name_matches = 0;
    let mut errors = Vec::new();

    // Try to resolve as a function call
    for function_no in function_nos {
        let func = &ns.functions[function_no];

        if func.name != id.name || func.ty != pt::FunctionTy::Function {
            continue;
        }

        name_matches += 1;

        let params_len = func.params.len();

        if params_len != args.len() {
            errors.push(Diagnostic::error(
                *loc,
                format!(
                    "function expects {} arguments, {} provided",
                    params_len,
                    args.len()
                ),
            ));
            continue;
        }

        let mut matches = true;
        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for i in 0..params_len {
            let param = &ns.functions[function_no].params[i];

            let arg = match arguments.get(&param.name) {
                Some(a) => a,
                None => {
                    matches = false;
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "missing argument ‘{}’ to function ‘{}’",
                            param.name, id.name,
                        ),
                    ));
                    break;
                }
            };

            let ty = param.ty.clone();

            let arg = match expression(
                arg,
                context,
                ns,
                symtable,
                &mut errors,
                ResolveTo::Type(&ty),
            ) {
                Ok(e) => e,
                Err(()) => {
                    matches = false;
                    continue;
                }
            };

            match cast(&arg.loc(), arg, &ty, true, ns, &mut errors) {
                Ok(expr) => cast_args.push(expr),
                Err(_) => {
                    matches = false;
                    continue;
                }
            }
        }

        if !matches {
            continue;
        }

        let func = &ns.functions[function_no];

        if func.contract_no != context.contract_no && func.is_private() {
            errors.push(Diagnostic::error_with_note(
                *loc,
                "cannot call private function".to_string(),
                func.loc,
                format!("declaration of function ‘{}’", func.name),
            ));

            continue;
        }

        let returns = function_returns(func);
        let ty = function_type(func, false);

        return Ok(Expression::InternalFunctionCall {
            loc: *loc,
            returns,
            function: Box::new(Expression::InternalFunction {
                loc: *loc,
                ty,
                function_no,
                signature: if virtual_call && (func.is_virtual || func.is_override.is_some()) {
                    Some(func.signature.clone())
                } else {
                    None
                },
            }),
            args: cast_args,
        });
    }

    match name_matches {
        0 => {
            diagnostics.push(Diagnostic::error(
                id.loc,
                format!("unknown function or type ‘{}’", id.name),
            ));
        }
        1 => diagnostics.extend(errors),
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "cannot find overloaded function which matches signature".to_string(),
            ));
        }
    }

    Err(())
}

/// Resolve a struct literal with named fields
fn named_struct_literal(
    loc: &pt::Loc,
    struct_no: usize,
    args: &[pt::NamedArgument],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let struct_def = ns.structs[struct_no].clone();

    if args.len() != struct_def.fields.len() {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "struct ‘{}’ has {} fields, not {}",
                struct_def.name,
                struct_def.fields.len(),
                args.len()
            ),
        ));
        Err(())
    } else {
        let mut fields = Vec::new();
        fields.resize(args.len(), Expression::Poison);
        for a in args {
            match struct_def
                .fields
                .iter()
                .enumerate()
                .find(|(_, f)| f.name == a.name.name)
            {
                Some((i, f)) => {
                    let expr = expression(
                        &a.expr,
                        context,
                        ns,
                        symtable,
                        diagnostics,
                        ResolveTo::Type(&f.ty),
                    )?;
                    used_variable(ns, &expr, symtable);
                    fields[i] = cast(loc, expr, &f.ty, true, ns, diagnostics)?;
                }
                None => {
                    diagnostics.push(Diagnostic::error(
                        a.name.loc,
                        format!(
                            "struct ‘{}’ has no field ‘{}’",
                            struct_def.name, a.name.name,
                        ),
                    ));
                    return Err(());
                }
            }
        }
        let ty = Type::Struct(struct_no);
        Ok(Expression::StructLiteral(*loc, ty, fields))
    }
}

/// Resolve a method call with positional arguments
fn method_call_pos_args(
    loc: &pt::Loc,
    var: &pt::Expression,
    func: &pt::Identifier,
    args: &[pt::Expression],
    call_args: &[&pt::NamedArgument],
    call_args_loc: Option<pt::Loc>,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    if let pt::Expression::Variable(namespace) = var {
        if builtin::is_builtin_call(Some(&namespace.name), &func.name, ns) {
            if let Some(loc) = call_args_loc {
                diagnostics.push(Diagnostic::error(
                    loc,
                    "call arguments not allowed on builtins".to_string(),
                ));
                return Err(());
            }

            return builtin::resolve_method_call(
                loc,
                &namespace.name,
                &func.name,
                args,
                context,
                ns,
                symtable,
                diagnostics,
            );
        }

        // is it a call to super
        if namespace.name == "super" {
            if let Some(cur_contract_no) = context.contract_no {
                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on super calls".to_string(),
                    ));
                    return Err(());
                }

                return call_position_args(
                    loc,
                    func,
                    pt::FunctionTy::Function,
                    args,
                    available_super_functions(&func.name, cur_contract_no, ns),
                    false,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                );
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "super not available outside contracts".to_string(),
                ));
                return Err(());
            }
        }

        // library or base contract call
        if let Some(call_contract_no) = ns.resolve_contract(context.file_no, namespace) {
            if ns.contracts[call_contract_no].is_library() {
                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on library calls".to_string(),
                    ));
                    return Err(());
                }

                return call_position_args(
                    loc,
                    func,
                    pt::FunctionTy::Function,
                    args,
                    available_functions(
                        &func.name,
                        false,
                        context.file_no,
                        Some(call_contract_no),
                        ns,
                    ),
                    true,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                );
            }

            // is a base contract of us
            if let Some(contract_no) = context.contract_no {
                if is_base(call_contract_no, contract_no, ns) {
                    if let Some(loc) = call_args_loc {
                        diagnostics.push(Diagnostic::error(
                            loc,
                            "call arguments not allowed on internal calls".to_string(),
                        ));
                        return Err(());
                    }

                    return call_position_args(
                        loc,
                        func,
                        pt::FunctionTy::Function,
                        args,
                        available_functions(
                            &func.name,
                            false,
                            context.file_no,
                            Some(call_contract_no),
                            ns,
                        ),
                        false,
                        context,
                        ns,
                        symtable,
                        diagnostics,
                    );
                }
            }
        }
    }

    let var_expr = expression(var, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
    let var_ty = var_expr.ty();

    if matches!(var_ty, Type::Bytes(_) | Type::String) && func.name == "format" {
        return if let pt::Expression::StringLiteral(bs) = var {
            if let Some(loc) = call_args_loc {
                diagnostics.push(Diagnostic::error(
                    loc,
                    "call arguments not allowed on builtins".to_string(),
                ));
                return Err(());
            }

            string_format(loc, bs, args, context, ns, symtable, diagnostics)
        } else {
            diagnostics.push(Diagnostic::error(
                *loc,
                "format only allowed on string literals".to_string(),
            ));
            Err(())
        };
    }

    if let Type::StorageRef(immutable, ty) = &var_ty {
        match ty.as_ref() {
            Type::Array(_, dim) => {
                if *immutable {
                    if let Some(function_no) = context.function_no {
                        if !ns.functions[function_no].is_constructor() {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                "cannot call method on immutable array outside of constructor"
                                    .to_string(),
                            ));
                            return Err(());
                        }
                    }
                }

                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on arrays".to_string(),
                    ));
                    return Err(());
                }

                if func.name == "push" {
                    if dim.last().unwrap().is_some() {
                        diagnostics.push(Diagnostic::error(
                            func.loc,
                            "method ‘push()’ not allowed on fixed length array".to_string(),
                        ));
                        return Err(());
                    }

                    let elem_ty = ty.array_elem();
                    let mut builtin_args = vec![var_expr];

                    let ret_ty = match args.len() {
                        1 => {
                            let expr = expression(
                                &args[0],
                                context,
                                ns,
                                symtable,
                                diagnostics,
                                ResolveTo::Type(&elem_ty),
                            )?;

                            builtin_args.push(cast(
                                &args[0].loc(),
                                expr,
                                &elem_ty,
                                true,
                                ns,
                                diagnostics,
                            )?);

                            Type::Void
                        }
                        0 => {
                            if elem_ty.is_reference_type() {
                                Type::StorageRef(false, Box::new(elem_ty))
                            } else {
                                elem_ty
                            }
                        }
                        _ => {
                            diagnostics.push(Diagnostic::error(
                                func.loc,
                                "method ‘push()’ takes at most 1 argument".to_string(),
                            ));
                            return Err(());
                        }
                    };

                    return Ok(Expression::Builtin(
                        func.loc,
                        vec![ret_ty],
                        Builtin::ArrayPush,
                        builtin_args,
                    ));
                }
                if func.name == "pop" {
                    if dim.last().unwrap().is_some() {
                        diagnostics.push(Diagnostic::error(
                            func.loc,
                            "method ‘pop()’ not allowed on fixed length array".to_string(),
                        ));

                        return Err(());
                    }

                    if !args.is_empty() {
                        diagnostics.push(Diagnostic::error(
                            func.loc,
                            "method ‘pop()’ does not take any arguments".to_string(),
                        ));
                        return Err(());
                    }

                    let storage_elem = ty.storage_array_elem();
                    let elem_ty = storage_elem.deref_any();

                    let return_ty = if resolve_to == ResolveTo::Discard {
                        Type::Void
                    } else {
                        elem_ty.clone()
                    };

                    return Ok(Expression::Builtin(
                        func.loc,
                        vec![return_ty],
                        Builtin::ArrayPop,
                        vec![var_expr],
                    ));
                }
            }
            Type::DynamicBytes => {
                if *immutable {
                    if let Some(function_no) = context.function_no {
                        if !ns.functions[function_no].is_constructor() {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                "cannot call method on immutable bytes outside of constructor"
                                    .to_string(),
                            ));
                            return Err(());
                        }
                    }
                }

                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on bytes".to_string(),
                    ));
                    return Err(());
                }

                if func.name == "push" {
                    let mut builtin_args = vec![var_expr];

                    let elem_ty = Type::Bytes(1);

                    let ret_ty = match args.len() {
                        1 => {
                            let expr = expression(
                                &args[0],
                                context,
                                ns,
                                symtable,
                                diagnostics,
                                ResolveTo::Type(&elem_ty),
                            )?;

                            builtin_args.push(cast(
                                &args[0].loc(),
                                expr,
                                &elem_ty,
                                true,
                                ns,
                                diagnostics,
                            )?);

                            Type::Void
                        }
                        0 => elem_ty,
                        _ => {
                            diagnostics.push(Diagnostic::error(
                                func.loc,
                                "method ‘push()’ takes at most 1 argument".to_string(),
                            ));
                            return Err(());
                        }
                    };
                    return Ok(Expression::Builtin(
                        func.loc,
                        vec![ret_ty],
                        Builtin::ArrayPush,
                        builtin_args,
                    ));
                }

                if func.name == "pop" {
                    if !args.is_empty() {
                        diagnostics.push(Diagnostic::error(
                            func.loc,
                            "method ‘pop()’ does not take any arguments".to_string(),
                        ));
                        return Err(());
                    }

                    return Ok(Expression::Builtin(
                        func.loc,
                        vec![Type::Bytes(1)],
                        Builtin::ArrayPop,
                        vec![var_expr],
                    ));
                }
            }
            _ => {}
        }
    }

    if matches!(var_ty, Type::Array(..) | Type::DynamicBytes) {
        if func.name == "push" {
            if ns.target == Target::Solana {
                diagnostics.push(Diagnostic::error(
                    func.loc,
                    format!("‘push()’ not supported on ‘bytes’ on target {}", ns.target),
                ));
                return Err(());
            }

            let elem_ty = match &var_ty {
                Type::Array(ty, _) => ty,
                Type::DynamicBytes => &Type::Uint(8),
                _ => unreachable!(),
            };
            let val = match args.len() {
                0 => elem_ty.default(ns).unwrap(),
                1 => {
                    let val_expr = expression(
                        &args[0],
                        context,
                        ns,
                        symtable,
                        diagnostics,
                        ResolveTo::Type(elem_ty),
                    )?;

                    cast(&args[0].loc(), val_expr, elem_ty, true, ns, diagnostics)?
                }
                _ => {
                    diagnostics.push(Diagnostic::error(
                        func.loc,
                        "method ‘push()’ takes at most 1 argument".to_string(),
                    ));
                    return Err(());
                }
            };

            return Ok(Expression::DynamicArrayPush(
                *loc,
                Box::new(var_expr),
                var_ty.clone(),
                Box::new(val),
            ));
        }
        if func.name == "pop" {
            if ns.target == Target::Solana {
                diagnostics.push(Diagnostic::error(
                    func.loc,
                    format!("‘pop()’ not supported on ‘bytes’ on target {}", ns.target),
                ));
                return Err(());
            }

            if !args.is_empty() {
                diagnostics.push(Diagnostic::error(
                    func.loc,
                    "method ‘pop()’ does not take any arguments".to_string(),
                ));
                return Err(());
            }

            return Ok(Expression::DynamicArrayPop(
                *loc,
                Box::new(var_expr),
                var_ty,
            ));
        }
    }

    if let Type::Contract(ext_contract_no) = &var_ty.deref_any() {
        let call_args = parse_call_args(call_args, true, context, ns, symtable, diagnostics)?;

        let marker = diagnostics.len();
        let mut name_match = 0;

        for function_no in ns.contracts[*ext_contract_no].functions.clone() {
            if func.name != ns.functions[function_no].name
                || ns.functions[function_no].ty != pt::FunctionTy::Function
            {
                continue;
            }

            name_match += 1;

            let params_len = ns.functions[function_no].params.len();

            if params_len != args.len() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "function expects {} arguments, {} provided",
                        params_len,
                        args.len()
                    ),
                ));
                continue;
            }
            let mut matches = true;
            let mut cast_args = Vec::new();
            // check if arguments can be implicitly casted
            for (i, arg) in args.iter().enumerate() {
                let ty = ns.functions[function_no].params[i].ty.clone();

                let arg = match expression(
                    arg,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&ty),
                ) {
                    Ok(e) => e,
                    Err(_) => {
                        matches = false;
                        continue;
                    }
                };

                match cast(&arg.loc(), arg, &ty, true, ns, diagnostics) {
                    Ok(expr) => cast_args.push(expr),
                    Err(()) => {
                        matches = false;
                        continue;
                    }
                }
            }
            if matches {
                diagnostics.truncate(marker);

                if !ns.functions[function_no].is_public() {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!("function ‘{}’ is not ‘public’ or ‘external’", func.name),
                    ));
                    return Err(());
                }

                let value = if let Some(value) = call_args.value {
                    if !value.const_zero(Some(*ext_contract_no), ns)
                        && !ns.functions[function_no].is_payable()
                    {
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "sending value to function ‘{}’ which is not payable",
                                func.name
                            ),
                        ));
                        return Err(());
                    }

                    Some(value)
                } else {
                    None
                };

                let func = &ns.functions[function_no];
                let returns = function_returns(func);
                let ty = function_type(func, true);

                return Ok(Expression::ExternalFunctionCall {
                    loc: *loc,
                    returns,
                    function: Box::new(Expression::ExternalFunction {
                        loc: *loc,
                        ty,
                        function_no,
                        address: Box::new(cast(
                            &var.loc(),
                            var_expr,
                            &Type::Contract(*ext_contract_no),
                            true,
                            ns,
                            diagnostics,
                        )?),
                    }),
                    args: cast_args,
                    value,
                    gas: call_args.gas,
                });
            }
        }

        let self_ty = var_ty.deref_any();

        // what about call args
        match resolve_using(
            loc,
            func,
            &var_expr,
            self_ty,
            context,
            args,
            symtable,
            diagnostics,
            ns,
        ) {
            Ok(Some(expr)) => {
                diagnostics.truncate(marker);

                return Ok(expr);
            }
            Ok(None) => (),
            Err(_) => {
                return Err(());
            }
        }

        if name_match != 1 {
            diagnostics.truncate(marker);
            diagnostics.push(Diagnostic::error(
                *loc,
                "cannot find overloaded function which matches signature".to_string(),
            ));
        }

        return Err(());
    }

    if let Type::Address(is_payable) = &var_ty.deref_any() {
        if func.name == "transfer" || func.name == "send" {
            if !is_payable {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "method ‘{}’ available on type ‘address payable’ not ‘address’",
                        func.name,
                    ),
                ));

                return Err(());
            }

            if args.len() != 1 {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "‘{}’ expects 1 argument, {} provided",
                        func.name,
                        args.len()
                    ),
                ));

                return Err(());
            }

            if let Some(loc) = call_args_loc {
                diagnostics.push(Diagnostic::error(
                    loc,
                    format!("call arguments not allowed on ‘{}’", func.name),
                ));
                return Err(());
            }

            let expr = expression(
                &args[0],
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&Type::Value),
            )?;

            let value = cast(&args[0].loc(), expr, &Type::Value, true, ns, diagnostics)?;

            return if func.name == "transfer" {
                Ok(Expression::Builtin(
                    *loc,
                    vec![Type::Void],
                    Builtin::PayableTransfer,
                    vec![var_expr, value],
                ))
            } else {
                Ok(Expression::Builtin(
                    *loc,
                    vec![Type::Bool],
                    Builtin::PayableSend,
                    vec![var_expr, value],
                ))
            };
        }
    }

    if let Type::Address(_) = &var_ty.deref_any() {
        let ty = match func.name.as_str() {
            "call" => Some(CallTy::Regular),
            "delegatecall" if ns.target == Target::Ewasm => Some(CallTy::Delegate),
            "staticcall" if ns.target == Target::Ewasm => Some(CallTy::Static),
            _ => None,
        };

        if let Some(ty) = ty {
            let call_args = parse_call_args(call_args, true, context, ns, symtable, diagnostics)?;

            if ty != CallTy::Regular && call_args.value.is_some() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!("‘{}’ cannot have value specifed", func.name,),
                ));

                return Err(());
            }

            if args.len() != 1 {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "‘{}’ expects 1 argument, {} provided",
                        func.name,
                        args.len()
                    ),
                ));

                return Err(());
            }

            let expr = expression(
                &args[0],
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&Type::DynamicBytes),
            )?;

            let args = cast(
                &args[0].loc(),
                expr,
                &Type::DynamicBytes,
                true,
                ns,
                diagnostics,
            )?;

            return Ok(Expression::ExternalFunctionCallRaw {
                loc: *loc,
                ty,
                args: Box::new(args),
                address: Box::new(var_expr),
                gas: call_args.gas,
                value: call_args.value,
            });
        }
    }

    // resolve it using library extension
    if context.contract_no.is_some() {
        let self_ty = var_ty.deref_any();

        match resolve_using(
            loc,
            func,
            &var_expr,
            self_ty,
            context,
            args,
            symtable,
            diagnostics,
            ns,
        ) {
            Ok(Some(expr)) => {
                return Ok(expr);
            }
            Ok(None) => (),
            Err(_) => {
                return Err(());
            }
        }
    }

    diagnostics.push(Diagnostic::error(
        func.loc,
        format!("method ‘{}’ does not exist", func.name),
    ));

    Err(())
}

fn resolve_using(
    loc: &pt::Loc,
    func: &pt::Identifier,
    self_expr: &Expression,
    self_ty: &Type,
    context: &ExprContext,
    args: &[pt::Expression],
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    ns: &mut Namespace,
) -> Result<Option<Expression>, ()> {
    // first collect all possible libraries that match the using directive type
    // Use HashSet for deduplication.
    // If the using directive specifies a type, the type must match the type of
    // the method call object exactly.
    let libraries: HashSet<usize> = ns.contracts[context.contract_no.unwrap()]
        .using
        .iter()
        .filter_map(|(library_no, ty)| match ty {
            None => Some(*library_no),
            Some(ty) if ty == self_ty => Some(*library_no),
            _ => None,
        })
        .collect();

    let mut name_matches = 0;
    let mut errors = Vec::new();

    for library_no in libraries {
        for function_no in ns.contracts[library_no].functions.clone() {
            let libfunc = &ns.functions[function_no];

            if libfunc.name != func.name || libfunc.ty != pt::FunctionTy::Function {
                continue;
            }

            name_matches += 1;

            let params_len = libfunc.params.len();

            if params_len != args.len() + 1 {
                errors.push(Diagnostic::error(
                    *loc,
                    format!(
                        "library function expects {} arguments, {} provided (including self)",
                        params_len,
                        args.len() + 1
                    ),
                ));
                continue;
            }
            let mut matches = true;
            let mut cast_args = Vec::new();

            match cast(
                &self_expr.loc(),
                self_expr.clone(),
                &libfunc.params[0].ty,
                true,
                ns,
                &mut errors,
            ) {
                Ok(e) => cast_args.push(e),
                Err(()) => continue,
            }

            // check if arguments can be implicitly casted
            for (i, arg) in args.iter().enumerate() {
                let ty = ns.functions[function_no].params[i + 1].ty.clone();

                let arg = match expression(
                    arg,
                    context,
                    ns,
                    symtable,
                    &mut errors,
                    ResolveTo::Type(&ty),
                ) {
                    Ok(e) => e,
                    Err(()) => {
                        matches = false;
                        continue;
                    }
                };

                match cast(&arg.loc(), arg.clone(), &ty, true, ns, &mut errors) {
                    Ok(expr) => cast_args.push(expr),
                    Err(_) => {
                        matches = false;
                        break;
                    }
                }
            }
            if !matches {
                continue;
            }

            let libfunc = &ns.functions[function_no];

            if libfunc.is_private() {
                errors.push(Diagnostic::error_with_note(
                    *loc,
                    "cannot call private library function".to_string(),
                    libfunc.loc,
                    format!("declaration of function ‘{}’", libfunc.name),
                ));

                continue;
            }

            let returns = function_returns(libfunc);
            let ty = function_type(libfunc, false);

            return Ok(Some(Expression::InternalFunctionCall {
                loc: *loc,
                returns,
                function: Box::new(Expression::InternalFunction {
                    loc: *loc,
                    ty,
                    function_no,
                    signature: None,
                }),
                args: cast_args,
            }));
        }
    }

    match name_matches {
        0 => Ok(None),
        1 => {
            diagnostics.extend(errors);

            Err(())
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "cannot find overloaded library function which matches signature".to_string(),
            ));
            Err(())
        }
    }
}

fn method_call_named_args(
    loc: &pt::Loc,
    var: &pt::Expression,
    func_name: &pt::Identifier,
    args: &[pt::NamedArgument],
    call_args: &[&pt::NamedArgument],
    call_args_loc: Option<pt::Loc>,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    if let pt::Expression::Variable(namespace) = var {
        // is it a call to super
        if namespace.name == "super" {
            if let Some(cur_contract_no) = context.contract_no {
                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on super calls".to_string(),
                    ));
                    return Err(());
                }

                return function_call_with_named_args(
                    loc,
                    func_name,
                    args,
                    available_super_functions(&func_name.name, cur_contract_no, ns),
                    false,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                );
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "super not available outside contracts".to_string(),
                ));
                return Err(());
            }
        }

        // library or base contract call
        if let Some(call_contract_no) = ns.resolve_contract(context.file_no, namespace) {
            if ns.contracts[call_contract_no].is_library() {
                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on library calls".to_string(),
                    ));
                    return Err(());
                }

                return function_call_with_named_args(
                    loc,
                    func_name,
                    args,
                    available_functions(
                        &func_name.name,
                        false,
                        context.file_no,
                        Some(call_contract_no),
                        ns,
                    ),
                    true,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                );
            }

            // is a base contract of us
            if let Some(contract_no) = context.contract_no {
                if is_base(call_contract_no, contract_no, ns) {
                    if let Some(loc) = call_args_loc {
                        diagnostics.push(Diagnostic::error(
                            loc,
                            "call arguments not allowed on internal calls".to_string(),
                        ));
                        return Err(());
                    }

                    return function_call_with_named_args(
                        loc,
                        func_name,
                        args,
                        available_functions(
                            &func_name.name,
                            false,
                            context.file_no,
                            Some(call_contract_no),
                            ns,
                        ),
                        false,
                        context,
                        ns,
                        symtable,
                        diagnostics,
                    );
                }
            }
        }
    }

    let var_expr = expression(var, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
    let var_ty = var_expr.ty();

    if let Type::Contract(external_contract_no) = &var_ty.deref_any() {
        let call_args = parse_call_args(call_args, true, context, ns, symtable, diagnostics)?;

        let mut arguments = HashMap::new();

        for arg in args {
            if arguments.contains_key(&arg.name.name) {
                diagnostics.push(Diagnostic::error(
                    arg.name.loc,
                    format!("duplicate argument with name ‘{}’", arg.name.name),
                ));
                return Err(());
            }

            arguments.insert(&arg.name.name, &arg.expr);
        }

        let marker = diagnostics.len();
        let mut name_match = 0;

        // function call
        for function_no in ns.contracts[*external_contract_no].functions.clone() {
            if ns.functions[function_no].name != func_name.name
                || ns.functions[function_no].ty != pt::FunctionTy::Function
            {
                continue;
            }

            let params_len = ns.functions[function_no].params.len();

            name_match += 1;

            if params_len != args.len() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "function expects {} arguments, {} provided",
                        params_len,
                        args.len()
                    ),
                ));
                continue;
            }
            let mut matches = true;
            let mut cast_args = Vec::new();
            // check if arguments can be implicitly casted
            for i in 0..params_len {
                let param = ns.functions[function_no].params[i].clone();

                let arg = match arguments.get(&param.name) {
                    Some(a) => a,
                    None => {
                        matches = false;
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "missing argument ‘{}’ to function ‘{}’",
                                param.name, func_name.name,
                            ),
                        ));
                        continue;
                    }
                };

                let arg = match expression(
                    arg,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&param.ty),
                ) {
                    Ok(e) => e,
                    Err(()) => {
                        matches = false;
                        continue;
                    }
                };

                match cast(&arg.loc(), arg.clone(), &param.ty, true, ns, diagnostics) {
                    Ok(expr) => cast_args.push(expr),
                    Err(()) => {
                        matches = false;
                        break;
                    }
                }
            }

            if matches {
                if !ns.functions[function_no].is_public() {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "function ‘{}’ is not ‘public’ or ‘external’",
                            func_name.name
                        ),
                    ));
                    return Err(());
                }

                let value = if let Some(value) = call_args.value {
                    if !value.const_zero(context.contract_no, ns)
                        && !ns.functions[function_no].is_payable()
                    {
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "sending value to function ‘{}’ which is not payable",
                                func_name.name
                            ),
                        ));
                        return Err(());
                    }

                    Some(value)
                } else {
                    None
                };

                let func = &ns.functions[function_no];
                let returns = function_returns(func);
                let ty = function_type(func, true);

                return Ok(Expression::ExternalFunctionCall {
                    loc: *loc,
                    returns,
                    function: Box::new(Expression::ExternalFunction {
                        loc: *loc,
                        ty,
                        function_no,
                        address: Box::new(cast(
                            &var.loc(),
                            var_expr,
                            &Type::Contract(*external_contract_no),
                            true,
                            ns,
                            diagnostics,
                        )?),
                    }),
                    args: cast_args,
                    value,
                    gas: call_args.gas,
                });
            }
        }

        match name_match {
            0 => {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "contract ‘{}’ does not have function ‘{}’",
                        var_ty.deref_any().to_string(ns),
                        func_name.name
                    ),
                ));
            }
            1 => {}
            _ => {
                diagnostics.truncate(marker);
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "cannot find overloaded function which matches signature".to_string(),
                ));
            }
        }
        return Err(());
    }

    diagnostics.push(Diagnostic::error(
        func_name.loc,
        format!("method ‘{}’ does not exist", func_name.name),
    ));

    Err(())
}

// When generating shifts, llvm wants both arguments to have the same width. We want the
// result of the shift to be left argument, so this function coercies the right argument
// into the right length.
pub fn cast_shift_arg(
    loc: &pt::Loc,
    expr: Expression,
    from_width: u16,
    ty: &Type,
    ns: &Namespace,
) -> Expression {
    let to_width = ty.bits(ns);

    if from_width == to_width {
        expr
    } else if from_width < to_width && ty.is_signed_int() {
        Expression::SignExt(*loc, ty.clone(), Box::new(expr))
    } else if from_width < to_width && !ty.is_signed_int() {
        Expression::ZeroExt(*loc, ty.clone(), Box::new(expr))
    } else {
        Expression::Trunc(*loc, ty.clone(), Box::new(expr))
    }
}

/// Given an parsed literal array, ensure that it is valid. All the elements in the array
/// must of the same type. The array might be a multidimensional array; all the leaf nodes
/// must match.
fn resolve_array_literal(
    loc: &pt::Loc,
    exprs: &[pt::Expression],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let mut dims = Box::new(Vec::new());
    let mut flattened = Vec::new();

    check_subarrays(exprs, &mut Some(&mut dims), &mut flattened, ns, diagnostics)?;

    if flattened.is_empty() {
        diagnostics.push(Diagnostic::error(
            *loc,
            "array requires at least one element".to_string(),
        ));
        return Err(());
    }

    let mut flattened = flattened.iter();

    let resolve_to = if let ResolveTo::Type(Type::Array(elem_ty, _)) = resolve_to {
        ResolveTo::Type(elem_ty)
    } else {
        resolve_to
    };

    // We follow the solidity scheme were everthing gets implicitly converted to the
    // type of the first element
    let first = expression(
        flattened.next().unwrap(),
        context,
        ns,
        symtable,
        diagnostics,
        resolve_to,
    )?;

    let ty = if let ResolveTo::Type(ty) = resolve_to {
        ty.clone()
    } else {
        first.ty()
    };

    used_variable(ns, &first, symtable);
    let mut exprs = vec![first];

    for e in flattened {
        let mut other = expression(e, context, ns, symtable, diagnostics, ResolveTo::Type(&ty))?;
        used_variable(ns, &other, symtable);

        if other.ty() != ty {
            other = cast(&e.loc(), other, &ty, true, ns, diagnostics)?;
        }

        exprs.push(other);
    }

    let aty = Type::Array(
        Box::new(ty),
        dims.iter()
            .map(|n| Some(BigInt::from_u32(*n).unwrap()))
            .collect::<Vec<Option<BigInt>>>(),
    );

    if context.constant {
        Ok(Expression::ConstArrayLiteral(*loc, aty, *dims, exprs))
    } else {
        Ok(Expression::ArrayLiteral(*loc, aty, *dims, exprs))
    }
}

/// Traverse the literal looking for sub arrays. Ensure that all the sub
/// arrays are the same length, and returned a flattened array of elements
fn check_subarrays<'a>(
    exprs: &'a [pt::Expression],
    dims: &mut Option<&mut Vec<u32>>,
    flatten: &mut Vec<&'a pt::Expression>,
    ns: &Namespace,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), ()> {
    if let Some(pt::Expression::ArrayLiteral(_, first)) = exprs.get(0) {
        // ensure all elements are array literals of the same length
        check_subarrays(first, dims, flatten, ns, diagnostics)?;

        for (i, e) in exprs.iter().enumerate().skip(1) {
            if let pt::Expression::ArrayLiteral(_, other) = e {
                if other.len() != first.len() {
                    diagnostics.push(Diagnostic::error(
                        e.loc(),
                        format!(
                            "array elements should be identical, sub array {} has {} elements rather than {}", i + 1, other.len(), first.len()
                        ),
                    ));
                    return Err(());
                }
                check_subarrays(other, &mut None, flatten, ns, diagnostics)?;
            } else {
                diagnostics.push(Diagnostic::error(
                    e.loc(),
                    format!("array element {} should also be an array", i + 1),
                ));
                return Err(());
            }
        }
    } else {
        for (i, e) in exprs.iter().enumerate().skip(1) {
            if let pt::Expression::ArrayLiteral(loc, _) = e {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "array elements should be of the type, element {} is unexpected array",
                        i + 1
                    ),
                ));
                return Err(());
            }
        }
        flatten.extend(exprs);
    }

    if let Some(dims) = dims.as_deref_mut() {
        dims.push(exprs.len() as u32);
    }

    Ok(())
}

/// Function call arguments
pub fn collect_call_args<'a>(
    expr: &'a pt::Expression,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<
    (
        &'a pt::Expression,
        Vec<&'a pt::NamedArgument>,
        Option<pt::Loc>,
    ),
    (),
> {
    let mut named_arguments = Vec::new();
    let mut expr = expr;
    let mut loc: Option<pt::Loc> = None;

    while let pt::Expression::FunctionCallBlock(_, e, block) = expr {
        match block.as_ref() {
            pt::Statement::Args(_, args) => {
                if let Some(l) = loc {
                    loc = Some(pt::Loc(l.0, l.1, block.loc().2));
                } else {
                    loc = Some(block.loc());
                }

                named_arguments.extend(args);
            }
            pt::Statement::Block { statements, .. } if statements.is_empty() => {
                // {}
                diagnostics.push(Diagnostic::error(
                    block.loc(),
                    "missing call arguments".to_string(),
                ));
                return Err(());
            }
            _ => {
                diagnostics.push(Diagnostic::error(
                    block.loc(),
                    "code block found where list of call arguments expected, like ‘{gas: 5000}’"
                        .to_string(),
                ));
                return Err(());
            }
        }

        expr = e;
    }

    Ok((expr, named_arguments, loc))
}

struct CallArgs {
    gas: Option<Box<Expression>>,
    salt: Option<Box<Expression>>,
    value: Option<Box<Expression>>,
    space: Option<Box<Expression>>,
}

/// Parse call arguments for external calls
fn parse_call_args(
    call_args: &[&pt::NamedArgument],
    external_call: bool,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<CallArgs, ()> {
    let mut args: HashMap<&String, &pt::NamedArgument> = HashMap::new();

    for arg in call_args {
        if let Some(prev) = args.get(&arg.name.name) {
            diagnostics.push(Diagnostic::error_with_note(
                arg.loc,
                format!("‘{}’ specified multiple times", arg.name.name),
                prev.loc,
                format!("location of previous declaration of ‘{}’", arg.name.name),
            ));
            return Err(());
        }

        args.insert(&arg.name.name, arg);
    }

    let mut res = CallArgs {
        gas: None,
        value: None,
        salt: None,
        space: None,
    };

    for arg in args.values() {
        match arg.name.name.as_str() {
            "value" => {
                let ty = Type::Value;

                let expr = expression(
                    &arg.expr,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&ty),
                )?;

                res.value = Some(Box::new(cast(
                    &arg.expr.loc(),
                    expr,
                    &ty,
                    true,
                    ns,
                    diagnostics,
                )?));
            }
            "gas" => {
                if ns.target == Target::Solana {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        format!(
                            "‘gas’ not permitted for external calls or constructors on {}",
                            ns.target
                        ),
                    ));
                    return Err(());
                }
                let ty = Type::Uint(64);

                let expr = expression(
                    &arg.expr,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&ty),
                )?;

                res.gas = Some(Box::new(cast(
                    &arg.expr.loc(),
                    expr,
                    &ty,
                    true,
                    ns,
                    diagnostics,
                )?));
            }
            "space" => {
                if ns.target != Target::Solana {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        format!(
                            "‘space’ not permitted for external calls or constructors on {}",
                            ns.target
                        ),
                    ));
                    return Err(());
                }

                if external_call {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        "‘space’ not valid for external calls".to_string(),
                    ));
                    return Err(());
                }

                let ty = Type::Uint(64);

                let expr = expression(
                    &arg.expr,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&ty),
                )?;

                res.space = Some(Box::new(cast(
                    &arg.expr.loc(),
                    expr,
                    &ty,
                    true,
                    ns,
                    diagnostics,
                )?));
            }
            "salt" => {
                if ns.target == Target::Solana {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        format!(
                            "‘salt’ not permitted for external calls or constructors on {}",
                            ns.target
                        ),
                    ));
                    return Err(());
                }

                if external_call {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        "‘salt’ not valid for external calls".to_string(),
                    ));
                    return Err(());
                }

                let ty = Type::Uint(256);

                let expr = expression(
                    &arg.expr,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&ty),
                )?;

                res.salt = Some(Box::new(cast(
                    &arg.expr.loc(),
                    expr,
                    &ty,
                    true,
                    ns,
                    diagnostics,
                )?));
            }
            _ => {
                diagnostics.push(Diagnostic::error(
                    arg.loc,
                    format!("‘{}’ not a valid call parameter", arg.name.name),
                ));
                return Err(());
            }
        }
    }

    Ok(res)
}

pub fn named_call_expr(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::NamedArgument],
    is_destructible: bool,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let mut nullsink = Vec::new();

    // is it a struct literal
    match ns.resolve_type(
        context.file_no,
        context.contract_no,
        true,
        ty,
        &mut nullsink,
    ) {
        Ok(Type::Struct(n)) => {
            return named_struct_literal(loc, n, args, context, ns, symtable, diagnostics);
        }
        Ok(_) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "struct or function expected".to_string(),
            ));
            return Err(());
        }
        _ => {}
    }

    // not a struct literal, remove those errors and try resolving as function call
    if context.constant {
        diagnostics.push(Diagnostic::error(
            *loc,
            "cannot call function in constant expression".to_string(),
        ));
        return Err(());
    }

    let expr = named_function_call_expr(loc, ty, args, context, ns, symtable, diagnostics)?;

    check_function_call(ns, &expr, symtable);
    if expr.tys().len() > 1 && !is_destructible {
        diagnostics.push(Diagnostic::error(
            *loc,
            "destucturing statement needed for function that returns multiple values".to_string(),
        ));
        return Err(());
    }

    Ok(expr)
}

/// Resolve any callable expression
pub fn call_expr(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::Expression],
    is_destructible: bool,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let mut nullsink = Vec::new();

    match ns.resolve_type(
        context.file_no,
        context.contract_no,
        true,
        ty,
        &mut nullsink,
    ) {
        Ok(Type::Struct(n)) => {
            return struct_literal(loc, n, args, context, ns, symtable, diagnostics);
        }
        Ok(to) => {
            // Cast
            return if args.is_empty() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "missing argument to cast".to_string(),
                ));
                Err(())
            } else if args.len() > 1 {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "too many arguments to cast".to_string(),
                ));
                Err(())
            } else {
                let expr = expression(
                    &args[0],
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Unknown,
                )?;

                cast(loc, expr, &to, false, ns, diagnostics)
            };
        }
        Err(_) => (),
    }

    let expr = function_call_expr(
        loc,
        ty,
        args,
        context,
        ns,
        symtable,
        diagnostics,
        resolve_to,
    )?;

    check_function_call(ns, &expr, symtable);
    if expr.tys().len() > 1 && !is_destructible {
        diagnostics.push(Diagnostic::error(
            *loc,
            "destucturing statement needed for function that returns multiple values".to_string(),
        ));
        return Err(());
    }

    Ok(expr)
}

/// Resolve function call
pub fn function_call_expr(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::Expression],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let (ty, call_args, call_args_loc) = collect_call_args(ty, diagnostics)?;

    match ty {
        pt::Expression::MemberAccess(_, member, func) => {
            if context.constant {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "cannot call function in constant expression".to_string(),
                ));
                return Err(());
            }

            method_call_pos_args(
                loc,
                member,
                func,
                args,
                &call_args,
                call_args_loc,
                context,
                ns,
                symtable,
                diagnostics,
                resolve_to,
            )
        }
        pt::Expression::Variable(id) => {
            // is it a builtin
            if builtin::is_builtin_call(None, &id.name, ns) {
                return {
                    let expr = builtin::resolve_call(
                        &id.loc,
                        None,
                        &id.name,
                        args,
                        context,
                        ns,
                        symtable,
                        diagnostics,
                    )?;

                    if expr.tys().len() > 1 {
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!("builtin function ‘{}’ returns more than one value", id.name),
                        ));
                        Err(())
                    } else {
                        Ok(expr)
                    }
                };
            }

            if context.constant {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "cannot call function in constant expression".to_string(),
                ));
                return Err(());
            }

            // is there a local variable or contract variable with this name
            if symtable.find(&id.name).is_some()
                || matches!(
                    ns.resolve_var(context.file_no, context.contract_no, id, true),
                    Some(Symbol::Variable(_, _, _))
                )
            {
                call_function_type(
                    loc,
                    ty,
                    args,
                    &call_args,
                    call_args_loc,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                )
            } else {
                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not permitted for internal calls".to_string(),
                    ));
                    return Err(());
                }

                call_position_args(
                    loc,
                    id,
                    pt::FunctionTy::Function,
                    args,
                    available_functions(&id.name, true, context.file_no, context.contract_no, ns),
                    true,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                )
            }
        }
        _ => call_function_type(
            loc,
            ty,
            args,
            &call_args,
            call_args_loc,
            context,
            ns,
            symtable,
            diagnostics,
        ),
    }
}

/// Resolve function call expression with named arguments
pub fn named_function_call_expr(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::NamedArgument],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let (ty, call_args, call_args_loc) = collect_call_args(ty, diagnostics)?;

    match ty {
        pt::Expression::MemberAccess(_, member, func) => method_call_named_args(
            loc,
            member,
            func,
            args,
            &call_args,
            call_args_loc,
            context,
            ns,
            symtable,
            diagnostics,
        ),
        pt::Expression::Variable(id) => {
            if let Some(loc) = call_args_loc {
                diagnostics.push(Diagnostic::error(
                    loc,
                    "call arguments not permitted for internal calls".to_string(),
                ));
                return Err(());
            }

            function_call_with_named_args(
                loc,
                id,
                args,
                available_functions(&id.name, true, context.file_no, context.contract_no, ns),
                true,
                context,
                ns,
                symtable,
                diagnostics,
            )
        }
        pt::Expression::ArraySubscript(_, _, _) => {
            diagnostics.push(Diagnostic::error(
                ty.loc(),
                "unexpected array type".to_string(),
            ));
            Err(())
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                ty.loc(),
                "expression not expected here".to_string(),
            ));
            Err(())
        }
    }
}

/// Get the return values for a function call
fn function_returns(ftype: &Function) -> Vec<Type> {
    if !ftype.returns.is_empty() {
        ftype.returns.iter().map(|p| p.ty.clone()).collect()
    } else {
        vec![Type::Void]
    }
}

/// Get the function type for an internal.external function call
fn function_type(func: &Function, external: bool) -> Type {
    let params = func.params.iter().map(|p| p.ty.clone()).collect();
    let mutability = func.mutability.clone();
    let returns = function_returns(func);

    if external {
        Type::ExternalFunction {
            params,
            mutability,
            returns,
        }
    } else {
        Type::InternalFunction {
            params,
            mutability,
            returns,
        }
    }
}

/// Calculate storage subscript
fn mapping_subscript(
    loc: &pt::Loc,
    mapping: Expression,
    index: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let ty = mapping.ty();

    if let Type::Mapping(key_ty, _) = ty.deref_any() {
        let index_expr = cast(
            &index.loc(),
            expression(
                index,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(key_ty),
            )?,
            key_ty,
            true,
            ns,
            diagnostics,
        )?;

        Ok(Expression::Subscript(
            *loc,
            ty,
            Box::new(mapping),
            Box::new(index_expr),
        ))
    } else {
        unreachable!()
    }
}
