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
use std::collections::HashMap;
use std::ops::Shl;
use std::ops::Sub;

use super::address::to_hexstr_eip55;
use super::ast::{
    Builtin, CallTy, ContractVariableType, Diagnostic, Expression, Function, Namespace,
    StringLocation, Symbol, Type,
};
use super::builtin;
use super::eval::eval_const_number;
use super::symtable::Symtable;
use crate::Target;
use hex;
use parser::pt;

impl Expression {
    /// Return the location for this expression
    pub fn loc(&self) -> pt::Loc {
        match self {
            Expression::FunctionArg(loc, _, _)
            | Expression::BoolLiteral(loc, _)
            | Expression::BytesLiteral(loc, _, _)
            | Expression::CodeLiteral(loc, _, _)
            | Expression::NumberLiteral(loc, _, _)
            | Expression::StructLiteral(loc, _, _)
            | Expression::ArrayLiteral(loc, _, _, _)
            | Expression::ConstArrayLiteral(loc, _, _, _)
            | Expression::Add(loc, _, _, _)
            | Expression::Subtract(loc, _, _, _)
            | Expression::Multiply(loc, _, _, _)
            | Expression::UDivide(loc, _, _, _)
            | Expression::SDivide(loc, _, _, _)
            | Expression::UModulo(loc, _, _, _)
            | Expression::SModulo(loc, _, _, _)
            | Expression::Power(loc, _, _, _)
            | Expression::BitwiseOr(loc, _, _, _)
            | Expression::BitwiseAnd(loc, _, _, _)
            | Expression::BitwiseXor(loc, _, _, _)
            | Expression::ShiftLeft(loc, _, _, _)
            | Expression::ShiftRight(loc, _, _, _, _)
            | Expression::Variable(loc, _, _)
            | Expression::ConstantVariable(loc, _, _, _)
            | Expression::StorageVariable(loc, _, _, _)
            | Expression::Load(loc, _, _)
            | Expression::StorageLoad(loc, _, _)
            | Expression::ZeroExt(loc, _, _)
            | Expression::SignExt(loc, _, _)
            | Expression::Trunc(loc, _, _)
            | Expression::Cast(loc, _, _)
            | Expression::UMore(loc, _, _)
            | Expression::ULess(loc, _, _)
            | Expression::UMoreEqual(loc, _, _)
            | Expression::ULessEqual(loc, _, _)
            | Expression::SMore(loc, _, _)
            | Expression::SLess(loc, _, _)
            | Expression::SMoreEqual(loc, _, _)
            | Expression::SLessEqual(loc, _, _)
            | Expression::Equal(loc, _, _)
            | Expression::NotEqual(loc, _, _)
            | Expression::Not(loc, _)
            | Expression::Complement(loc, _, _)
            | Expression::UnaryMinus(loc, _, _)
            | Expression::Ternary(loc, _, _, _, _)
            | Expression::ArraySubscript(loc, _, _, _)
            | Expression::StructMember(loc, _, _, _)
            | Expression::Or(loc, _, _)
            | Expression::AllocDynamicArray(loc, _, _, _)
            | Expression::DynamicArrayLength(loc, _)
            | Expression::DynamicArraySubscript(loc, _, _, _)
            | Expression::DynamicArrayPush(loc, _, _, _)
            | Expression::DynamicArrayPop(loc, _, _)
            | Expression::StorageBytesSubscript(loc, _, _)
            | Expression::StorageBytesPush(loc, _, _)
            | Expression::StorageBytesPop(loc, _)
            | Expression::StorageBytesLength(loc, _)
            | Expression::StringCompare(loc, _, _)
            | Expression::StringConcat(loc, _, _, _)
            | Expression::Keccak256(loc, _, _)
            | Expression::ReturnData(loc)
            | Expression::InternalFunctionCall(loc, _, _, _)
            | Expression::ExternalFunctionCall { loc, .. }
            | Expression::ExternalFunctionCallRaw { loc, .. }
            | Expression::Constructor { loc, .. }
            | Expression::GetAddress(loc, _)
            | Expression::Balance(loc, _, _)
            | Expression::PreIncrement(loc, _, _)
            | Expression::PreDecrement(loc, _, _)
            | Expression::PostIncrement(loc, _, _)
            | Expression::PostDecrement(loc, _, _)
            | Expression::Builtin(loc, _, _, _)
            | Expression::Assign(loc, _, _, _)
            | Expression::List(loc, _)
            | Expression::And(loc, _, _) => *loc,
            Expression::Poison => unreachable!(),
        }
    }

    /// Return the type for this expression. This assumes the expression has a single value,
    /// panics will occur otherwise
    pub fn ty(&self) -> Type {
        match self {
            Expression::BoolLiteral(_, _)
            | Expression::UMore(_, _, _)
            | Expression::ULess(_, _, _)
            | Expression::UMoreEqual(_, _, _)
            | Expression::ULessEqual(_, _, _)
            | Expression::SMore(_, _, _)
            | Expression::SLess(_, _, _)
            | Expression::SMoreEqual(_, _, _)
            | Expression::SLessEqual(_, _, _)
            | Expression::Equal(_, _, _)
            | Expression::Or(_, _, _)
            | Expression::And(_, _, _)
            | Expression::NotEqual(_, _, _)
            | Expression::Not(_, _)
            | Expression::StringCompare(_, _, _) => Type::Bool,
            Expression::CodeLiteral(_, _, _) => Type::DynamicBytes,
            Expression::StringConcat(_, ty, _, _)
            | Expression::FunctionArg(_, ty, _)
            | Expression::BytesLiteral(_, ty, _)
            | Expression::NumberLiteral(_, ty, _)
            | Expression::StructLiteral(_, ty, _)
            | Expression::ArrayLiteral(_, ty, _, _)
            | Expression::ConstArrayLiteral(_, ty, _, _)
            | Expression::Add(_, ty, _, _)
            | Expression::Subtract(_, ty, _, _)
            | Expression::Multiply(_, ty, _, _)
            | Expression::UDivide(_, ty, _, _)
            | Expression::SDivide(_, ty, _, _)
            | Expression::UModulo(_, ty, _, _)
            | Expression::SModulo(_, ty, _, _)
            | Expression::Power(_, ty, _, _)
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
            | Expression::Complement(_, ty, _)
            | Expression::UnaryMinus(_, ty, _)
            | Expression::Ternary(_, ty, _, _, _)
            | Expression::ArraySubscript(_, ty, _, _)
            | Expression::StructMember(_, ty, _, _)
            | Expression::AllocDynamicArray(_, ty, _, _)
            | Expression::DynamicArraySubscript(_, ty, _, _)
            | Expression::Balance(_, ty, _)
            | Expression::PreIncrement(_, ty, _)
            | Expression::PreDecrement(_, ty, _)
            | Expression::PostIncrement(_, ty, _)
            | Expression::PostDecrement(_, ty, _)
            | Expression::GetAddress(_, ty)
            | Expression::Keccak256(_, ty, _)
            | Expression::Assign(_, ty, _, _) => ty.clone(),
            Expression::DynamicArrayPush(_, _, ty, _) | Expression::DynamicArrayPop(_, _, ty) => {
                match ty {
                    Type::Array(..) => ty.array_elem(),
                    Type::DynamicBytes => Type::Uint(8),
                    _ => unreachable!(),
                }
            }
            Expression::DynamicArrayLength(_, _) => Type::Uint(32),
            Expression::StorageBytesLength(_, _) => Type::Uint(32),
            Expression::StorageBytesSubscript(_, _, _) => {
                Type::StorageRef(Box::new(Type::Bytes(1)))
            }
            Expression::ExternalFunctionCallRaw { .. } => {
                panic!("two return values");
            }
            Expression::Builtin(_, returns, _, _)
            | Expression::InternalFunctionCall(_, returns, _, _)
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
            // codegen Expressions
            Expression::ReturnData(_) => Type::DynamicBytes,
            Expression::StorageBytesPush(_, _, _) | Expression::StorageBytesPop(_, _) => {
                unreachable!()
            }
        }
    }
    /// Is this expression 0
    fn const_zero(&self, contract_no: Option<usize>, ns: &mut Namespace) -> bool {
        if let Ok((_, value)) = eval_const_number(&self, contract_no, ns) {
            value == BigInt::zero()
        } else {
            false
        }
    }

    /// Get the returns for a function call
    pub fn tys(&self) -> Vec<Type> {
        match self {
            Expression::Builtin(_, returns, _, _)
            | Expression::InternalFunctionCall(_, returns, _, _)
            | Expression::ExternalFunctionCall { returns, .. } => returns.to_vec(),
            Expression::List(_, list) => list.iter().map(|e| e.ty()).collect(),
            Expression::ExternalFunctionCallRaw { .. } => vec![Type::Bool, Type::DynamicBytes],
            Expression::DynamicArrayPush(_, _, ty, _) | Expression::DynamicArrayPop(_, _, ty) => {
                match ty {
                    Type::Array(..) => vec![ty.array_elem()],
                    Type::DynamicBytes => vec![Type::Uint(8)],
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }
}

/// Unescape a string literal
fn unescape(literal: &str, start: usize, file_no: usize, ns: &mut Namespace) -> String {
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
                        ns.diagnostics.push(Diagnostic::error(
                            pt::Loc(file_no, start + i, start + i + 4),
                            format!("\\x{:02x} is not a valid unicode character", ch),
                        ));
                    }
                },
                Err(offset) => {
                    ns.diagnostics.push(Diagnostic::error(
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
                        ns.diagnostics.push(Diagnostic::error(
                            pt::Loc(file_no, start + i, start + i + 6),
                            format!("\\u{:04x} is not a valid unicode character", ch),
                        ));
                    }
                },
                Err(offset) => {
                    ns.diagnostics.push(Diagnostic::error(
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
                ns.diagnostics.push(Diagnostic::error(
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
    ns: &mut Namespace,
) -> Result<Type, ()> {
    let l = match l {
        Type::Ref(ty) => ty,
        Type::StorageRef(ty) => ty,
        _ => l,
    };
    let r = match r {
        Type::Ref(ty) => ty,
        Type::StorageRef(ty) => ty,
        _ => r,
    };

    if *l == *r {
        return Ok(l.clone());
    }

    coerce_int(l, l_loc, r, r_loc, true, ns)
}

fn get_int_length(
    l: &Type,
    l_loc: &pt::Loc,
    allow_bytes: bool,
    ns: &mut Namespace,
) -> Result<(u16, bool), ()> {
    match l {
        Type::Uint(n) => Ok((*n, false)),
        Type::Int(n) => Ok((*n, true)),
        Type::Bytes(n) if allow_bytes => Ok((*n as u16 * 8, false)),
        Type::Enum(n) => {
            ns.diagnostics.push(Diagnostic::error(
                *l_loc,
                format!("type enum {} not allowed", ns.enums[*n].print_to_string(),),
            ));
            Err(())
        }
        Type::Struct(n) => {
            ns.diagnostics.push(Diagnostic::error(
                *l_loc,
                format!(
                    "type struct {} not allowed",
                    ns.structs[*n].print_to_string()
                ),
            ));
            Err(())
        }
        Type::Array(_, _) => {
            ns.diagnostics.push(Diagnostic::error(
                *l_loc,
                format!("type array {} not allowed", l.to_string(ns)),
            ));
            Err(())
        }
        Type::Ref(n) => get_int_length(n, l_loc, allow_bytes, ns),
        Type::StorageRef(n) => get_int_length(n, l_loc, allow_bytes, ns),
        _ => {
            ns.diagnostics.push(Diagnostic::error(
                *l_loc,
                format!("expression of type {} not allowed", l.to_string(ns)),
            ));
            Err(())
        }
    }
}

fn coerce_int(
    l: &Type,
    l_loc: &pt::Loc,
    r: &Type,
    r_loc: &pt::Loc,
    allow_bytes: bool,
    ns: &mut Namespace,
) -> Result<Type, ()> {
    let l = match l {
        Type::Ref(ty) => ty,
        Type::StorageRef(ty) => ty,
        _ => l,
    };
    let r = match r {
        Type::Ref(ty) => ty,
        Type::StorageRef(ty) => ty,
        _ => r,
    };

    match (l, r) {
        (Type::Bytes(left_length), Type::Bytes(right_length)) if allow_bytes => {
            return Ok(Type::Bytes(std::cmp::max(*left_length, *right_length)));
        }
        _ => (),
    }

    let (left_len, left_signed) = get_int_length(l, l_loc, false, ns)?;

    let (right_len, right_signed) = get_int_length(r, r_loc, false, ns)?;

    Ok(match (left_signed, right_signed) {
        (true, true) => Type::Int(cmp::max(left_len, right_len)),
        (false, false) => Type::Uint(cmp::max(left_len, right_len)),
        (true, false) => Type::Int(cmp::max(left_len, cmp::min(right_len + 8, 256))),
        (false, true) => Type::Int(cmp::max(cmp::min(left_len + 8, 256), right_len)),
    })
}

/// Try to convert a BigInt into a Expression::NumberLiteral. This checks for sign,
/// width and creates to correct Type.
fn bigint_to_expression(loc: &pt::Loc, n: &BigInt, ns: &mut Namespace) -> Result<Expression, ()> {
    try_bigint_to_expression(loc, n).map_err(|d| {
        ns.diagnostics.push(d);
    })
}

pub fn try_bigint_to_expression(loc: &pt::Loc, n: &BigInt) -> Result<Expression, Diagnostic> {
    // Return smallest type
    let bits = n.bits();

    let int_size = if bits < 7 { 8 } else { (bits + 7) & !7 } as u16;

    if n.sign() == Sign::Minus {
        if bits > 255 {
            Err(Diagnostic::error(*loc, format!("{} is too large", n)))
        } else {
            Ok(Expression::NumberLiteral(
                *loc,
                Type::Int(int_size),
                n.clone(),
            ))
        }
    } else if bits > 256 {
        Err(Diagnostic::error(*loc, format!("{} is too large", n)))
    } else {
        Ok(Expression::NumberLiteral(
            *loc,
            Type::Uint(int_size),
            n.clone(),
        ))
    }
}

pub fn cast(
    loc: &pt::Loc,
    expr: Expression,
    to: &Type,
    implicit: bool,
    ns: &mut Namespace,
) -> Result<Expression, ()> {
    try_cast(loc, expr, to, implicit, ns).map_err(|diagnostic| {
        ns.diagnostics.push(diagnostic);
    })
}

/// Cast from one type to another, which also automatically derefs any Type::Ref() type.
/// if the cast is explicit (e.g. bytes32(bar) then implicit should be set to false.
pub fn try_cast(
    loc: &pt::Loc,
    expr: Expression,
    to: &Type,
    implicit: bool,
    ns: &Namespace,
) -> Result<Expression, Diagnostic> {
    let from = expr.ty();

    if &from == to {
        return Ok(expr);
    }

    // First of all, if we have a ref then derefence it
    if let Type::Ref(r) = from {
        return try_cast(
            loc,
            Expression::Load(*loc, r.as_ref().clone(), Box::new(expr)),
            to,
            implicit,
            ns,
        );
    }

    // If it's a storage reference then load the value. The expr is the storage slot
    if let Type::StorageRef(r) = from {
        if let Expression::StorageBytesSubscript(_, _, _) = expr {
            return Ok(expr);
        } else {
            return try_cast(
                loc,
                Expression::StorageLoad(*loc, *r, Box::new(expr)),
                to,
                implicit,
                ns,
            );
        }
    }

    // Special case: when converting literal sign can change if it fits
    match (&expr, &from, to) {
        (&Expression::NumberLiteral(_, _, ref n), p, &Type::Uint(to_len)) if p.is_primitive() => {
            return if n.sign() == Sign::Minus {
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion cannot change negative number to {}",
                        to.to_string(ns)
                    ),
                ))
            } else if n.bits() >= to_len as u64 {
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion would truncate from {} to {}",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ))
            } else {
                Ok(Expression::NumberLiteral(
                    *loc,
                    Type::Uint(to_len),
                    n.clone(),
                ))
            }
        }
        (&Expression::NumberLiteral(_, _, ref n), p, &Type::Int(to_len)) if p.is_primitive() => {
            return if n.bits() >= to_len as u64 {
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion would truncate from {} to {}",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ))
            } else {
                Ok(Expression::NumberLiteral(
                    *loc,
                    Type::Int(to_len),
                    n.clone(),
                ))
            }
        }
        // Literal strings can be implicitly lengthened
        (&Expression::BytesLiteral(_, _, ref bs), p, &Type::Bytes(to_len)) if p.is_primitive() => {
            return if bs.len() > to_len as usize && implicit {
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion would truncate from {} to {}",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ))
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
        _ => (),
    };

    cast_types(loc, expr, from, to.clone(), implicit, ns)
}

/// Do casting between types (no literals)
fn cast_types(
    loc: &pt::Loc,
    expr: Expression,
    from: Type,
    to: Type,
    implicit: bool,
    ns: &Namespace,
) -> Result<Expression, Diagnostic> {
    let address_bits = ns.address_length as u16 * 8;

    #[allow(clippy::comparison_chain)]
    match (&from, &to) {
        (Type::Uint(from_width), Type::Enum(enum_no))
        | (Type::Int(from_width), Type::Enum(enum_no)) => {
            if implicit {
                return Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion from {} to {} not allowed",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));
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

                return Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "enum {} has no value with ordinal {}",
                        to.to_string(ns),
                        big_number
                    ),
                ));
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
                return Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion from {} to {} not allowed",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));
            }

            let enum_ty = &ns.enums[*enum_no];
            let from_width = enum_ty.ty.bits(ns);

            match from_width.cmp(&to_width) {
                Ordering::Greater => Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr))),
                Ordering::Less => Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr))),
                Ordering::Equal => Ok(Expression::Cast(*loc, to.clone(), Box::new(expr))),
            }
        }
        (Type::Bytes(1), Type::Uint(8)) => Ok(expr),
        (Type::Uint(8), Type::Bytes(1)) => Ok(expr),
        (Type::Uint(from_len), Type::Uint(to_len)) => match from_len.cmp(&to_len) {
            Ordering::Greater => {
                if implicit {
                    Err(Diagnostic::type_error(
                        *loc,
                        format!(
                            "implicit conversion would truncate from {} to {}",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ))
                } else {
                    Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
                }
            }
            Ordering::Less => Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr))),
            Ordering::Equal => Ok(Expression::Cast(*loc, to.clone(), Box::new(expr))),
        },
        (Type::Int(from_len), Type::Int(to_len)) => match from_len.cmp(&to_len) {
            Ordering::Greater => {
                if implicit {
                    Err(Diagnostic::type_error(
                        *loc,
                        format!(
                            "implicit conversion would truncate from {} to {}",
                            from.to_string(ns),
                            to.to_string(ns)
                        ),
                    ))
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
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion would change sign from {} to {}",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ))
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
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion would change sign from {} to {}",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ))
            } else if from_len > to_len {
                Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
            } else if from_len < to_len {
                Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr)))
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        // Casting int to address
        (Type::Uint(from_len), Type::Address(_)) | (Type::Int(from_len), Type::Address(_)) => {
            if implicit {
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion from {} to address not allowed",
                        from.to_string(ns)
                    ),
                ))
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
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from address not allowed",
                        from.to_string(ns)
                    ),
                ))
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
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion would truncate from {} to {}",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ))
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
        // Explicit conversion from bytesN to int/uint only allowed with expliciy
        // cast and if it is the same size (i.e. no conversion required)
        (Type::Bytes(from_len), Type::Uint(to_len))
        | (Type::Bytes(from_len), Type::Int(to_len)) => {
            if implicit {
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ))
            } else if *from_len as u16 * 8 != *to_len {
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ))
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        // Explicit conversion to bytesN from int/uint only allowed with expliciy
        // cast and if it is the same size (i.e. no conversion required)
        (Type::Uint(from_len), Type::Bytes(to_len))
        | (Type::Int(from_len), Type::Bytes(to_len)) => {
            if implicit {
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ))
            } else if *to_len as u16 * 8 != *from_len {
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ))
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        // Explicit conversion from bytesN to address only allowed with expliciy
        // cast and if it is the same size (i.e. no conversion required)
        (Type::Bytes(from_len), Type::Address(_)) => {
            if implicit {
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ))
            } else if *from_len as usize != ns.address_length {
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ))
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        // Explicit conversion between contract and address is allowed
        (Type::Address(false), Type::Address(true))
        | (Type::Address(_), Type::Contract(_))
        | (Type::Contract(_), Type::Address(_)) => {
            if implicit {
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ))
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
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ))
            } else if *to_len as usize != ns.address_length {
                Err(Diagnostic::type_error(
                    *loc,
                    format!(
                        "conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ))
            } else {
                Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
            }
        }
        (Type::String, Type::DynamicBytes) | (Type::DynamicBytes, Type::String) if !implicit => {
            Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)))
        }
        // string conversions
        /*
        (Type::Bytes(_), Type::String) => Ok(Expression::Cast(*loc, to.clone(), Box::new(expr)),
        (Type::String, Type::Bytes(to_len)) => {
            if let Expression::BytesLiteral(_, from_str) = &expr {
                if from_str.len() > to_len as usize {
                    ns.diagnostics.push(Output::type_error(
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
        (Type::Void, _) => Err(Diagnostic::type_error(
            *loc,
            "function or method does not return a value".to_string(),
        )),
        _ => Err(Diagnostic::type_error(
            *loc,
            format!(
                "conversion from {} to {} not possible",
                from.to_string(ns),
                to.to_string(ns)
            ),
        )),
    }
}

/// Resolve a parsed expression into an AST expression
pub fn expression(
    expr: &pt::Expression,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
    is_constant: bool,
) -> Result<Expression, ()> {
    match expr {
        pt::Expression::ArrayLiteral(loc, exprs) => {
            resolve_array_literal(loc, exprs, file_no, contract_no, ns, symtable, is_constant)
        }
        pt::Expression::BoolLiteral(loc, v) => Ok(Expression::BoolLiteral(*loc, *v)),
        pt::Expression::StringLiteral(v) => {
            // Concatenate the strings
            let mut result = Vec::new();
            let mut loc = v[0].loc;

            for s in v {
                result.extend_from_slice(unescape(&s.string, s.loc.1, file_no, ns).as_bytes());
                loc.2 = s.loc.2;
            }

            let length = result.len();

            Ok(Expression::BytesLiteral(
                loc,
                Type::Bytes(length as u8),
                result,
            ))
        }
        pt::Expression::HexLiteral(v) => {
            let mut result = Vec::new();
            let mut loc = v[0].loc;

            for s in v {
                if (s.hex.len() % 2) != 0 {
                    ns.diagnostics.push(Diagnostic::error(
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
        pt::Expression::NumberLiteral(loc, b) => bigint_to_expression(loc, b, ns),
        pt::Expression::HexNumberLiteral(loc, n) => {
            // ns.address_length is in bytes; double for hex and two for the leading 0x
            let looks_like_address = n.len() == ns.address_length * 2 + 2
                && n.starts_with("0x")
                && !n.chars().any(|c| c == '_');

            if looks_like_address {
                let address = to_hexstr_eip55(n);

                if address == *n {
                    let s: String = address.chars().skip(2).collect();

                    Ok(Expression::NumberLiteral(
                        *loc,
                        Type::Address(false),
                        BigInt::from_str_radix(&s, 16).unwrap(),
                    ))
                } else {
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "address literal has incorrect checksum, expected ‘{}’",
                            address
                        ),
                    ));
                    Err(())
                }
            } else {
                // from_str_radix does not like the 0x prefix
                let s: String = n.chars().filter(|v| *v != 'x' && *v != '_').collect();

                bigint_to_expression(loc, &BigInt::from_str_radix(&s, 16).unwrap(), ns)
            }
        }
        pt::Expression::Variable(id) => {
            if let Some(v) = symtable.find(&id.name) {
                return if is_constant {
                    ns.diagnostics.push(Diagnostic::error(
                        id.loc,
                        format!("cannot read variable ‘{}’ in constant expression", id.name),
                    ));
                    Err(())
                } else {
                    Ok(Expression::Variable(id.loc, v.ty.clone(), v.pos))
                };
            }

            if let Some((builtin, ty)) = builtin::builtin_var(None, &id.name, ns) {
                return Ok(Expression::Builtin(id.loc, vec![ty], builtin, vec![]));
            }

            let (var_contract_no, var_no) = ns.resolve_var(file_no, contract_no.unwrap(), id)?;

            let var = &ns.contracts[var_contract_no].variables[var_no];

            match var.var {
                ContractVariableType::Constant => Ok(Expression::ConstantVariable(
                    id.loc,
                    var.ty.clone(),
                    var_contract_no,
                    var_no,
                )),
                ContractVariableType::Storage => {
                    if is_constant {
                        ns.diagnostics.push(Diagnostic::error(
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
                            Type::StorageRef(Box::new(var.ty.clone())),
                            var_contract_no,
                            var_no,
                        ))
                    }
                }
            }
        }
        pt::Expression::Add(loc, l, r) => {
            addition(loc, l, r, file_no, contract_no, ns, symtable, is_constant)
        }
        pt::Expression::Subtract(loc, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

            let ty = coerce_int(&left.ty(), &l.loc(), &right.ty(), &r.loc(), false, ns)?;

            Ok(Expression::Subtract(
                *loc,
                ty.clone(),
                Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                Box::new(cast(&r.loc(), right, &ty, true, ns)?),
            ))
        }
        pt::Expression::BitwiseOr(loc, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

            let ty = coerce_int(&left.ty(), &l.loc(), &right.ty(), &r.loc(), true, ns)?;

            Ok(Expression::BitwiseOr(
                *loc,
                ty.clone(),
                Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                Box::new(cast(&r.loc(), right, &ty, true, ns)?),
            ))
        }
        pt::Expression::BitwiseAnd(loc, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

            let ty = coerce_int(&left.ty(), &l.loc(), &right.ty(), &r.loc(), true, ns)?;

            Ok(Expression::BitwiseAnd(
                *loc,
                ty.clone(),
                Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                Box::new(cast(&r.loc(), right, &ty, true, ns)?),
            ))
        }
        pt::Expression::BitwiseXor(loc, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

            let ty = coerce_int(&left.ty(), &l.loc(), &right.ty(), &r.loc(), true, ns)?;

            Ok(Expression::BitwiseXor(
                *loc,
                ty.clone(),
                Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                Box::new(cast(&r.loc(), right, &ty, true, ns)?),
            ))
        }
        pt::Expression::ShiftLeft(loc, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

            // left hand side may be bytes/int/uint
            // right hand size may be int/uint
            let _ = get_int_length(&left.ty(), &l.loc(), true, ns)?;
            let (right_length, _) = get_int_length(&right.ty(), &r.loc(), false, ns)?;

            let left_type = left.ty();

            Ok(Expression::ShiftLeft(
                *loc,
                left_type.clone(),
                Box::new(left),
                Box::new(cast_shift_arg(loc, right, right_length, &left_type, ns)),
            ))
        }
        pt::Expression::ShiftRight(loc, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

            let left_type = left.ty();
            // left hand side may be bytes/int/uint
            // right hand size may be int/uint
            let _ = get_int_length(&left_type, &l.loc(), true, ns)?;
            let (right_length, _) = get_int_length(&right.ty(), &r.loc(), false, ns)?;

            Ok(Expression::ShiftRight(
                *loc,
                left_type.clone(),
                Box::new(left),
                Box::new(cast_shift_arg(loc, right, right_length, &left_type, ns)),
                left_type.is_signed_int(),
            ))
        }
        pt::Expression::Multiply(loc, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

            let ty = coerce_int(&left.ty(), &l.loc(), &right.ty(), &r.loc(), false, ns)?;

            Ok(Expression::Multiply(
                *loc,
                ty.clone(),
                Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                Box::new(cast(&r.loc(), right, &ty, true, ns)?),
            ))
        }
        pt::Expression::Divide(loc, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

            let ty = coerce_int(&left.ty(), &l.loc(), &right.ty(), &r.loc(), false, ns)?;

            if ty.is_signed_int() {
                Ok(Expression::SDivide(
                    *loc,
                    ty.clone(),
                    Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                    Box::new(cast(&r.loc(), right, &ty, true, ns)?),
                ))
            } else {
                Ok(Expression::UDivide(
                    *loc,
                    ty.clone(),
                    Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                    Box::new(cast(&r.loc(), right, &ty, true, ns)?),
                ))
            }
        }
        pt::Expression::Modulo(loc, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

            let ty = coerce_int(&left.ty(), &l.loc(), &right.ty(), &r.loc(), false, ns)?;

            if ty.is_signed_int() {
                Ok(Expression::SModulo(
                    *loc,
                    ty.clone(),
                    Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                    Box::new(cast(&r.loc(), right, &ty, true, ns)?),
                ))
            } else {
                Ok(Expression::UModulo(
                    *loc,
                    ty.clone(),
                    Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                    Box::new(cast(&r.loc(), right, &ty, true, ns)?),
                ))
            }
        }
        pt::Expression::Power(loc, b, e) => {
            let base = expression(b, file_no, contract_no, ns, symtable, is_constant)?;
            let exp = expression(e, file_no, contract_no, ns, symtable, is_constant)?;

            let base_type = base.ty();
            let exp_type = exp.ty();

            // solc-0.5.13 does not allow either base or exp to be signed
            if base_type.is_signed_int() || exp_type.is_signed_int() {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "exponation (**) is not allowed with signed types".to_string(),
                ));
                return Err(());
            }

            let ty = coerce_int(&base_type, &b.loc(), &exp_type, &e.loc(), false, ns)?;

            Ok(Expression::Power(
                *loc,
                ty.clone(),
                Box::new(cast(&b.loc(), base, &ty, true, ns)?),
                Box::new(cast(&e.loc(), exp, &ty, true, ns)?),
            ))
        }

        // compare
        pt::Expression::More(loc, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

            let ty = coerce_int(&left.ty(), &l.loc(), &right.ty(), &r.loc(), true, ns)?;

            if ty.is_signed_int() {
                Ok(Expression::SMore(
                    *loc,
                    Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                    Box::new(cast(&r.loc(), right, &ty, true, ns)?),
                ))
            } else {
                Ok(Expression::UMore(
                    *loc,
                    Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                    Box::new(cast(&r.loc(), right, &ty, true, ns)?),
                ))
            }
        }
        pt::Expression::Less(loc, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

            let ty = coerce_int(&left.ty(), &l.loc(), &right.ty(), &r.loc(), true, ns)?;

            if ty.is_signed_int() {
                Ok(Expression::SLess(
                    *loc,
                    Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                    Box::new(cast(&r.loc(), right, &ty, true, ns)?),
                ))
            } else {
                Ok(Expression::ULess(
                    *loc,
                    Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                    Box::new(cast(&r.loc(), right, &ty, true, ns)?),
                ))
            }
        }
        pt::Expression::MoreEqual(loc, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

            let ty = coerce_int(&left.ty(), &l.loc(), &right.ty(), &r.loc(), true, ns)?;

            if ty.is_signed_int() {
                Ok(Expression::SMoreEqual(
                    *loc,
                    Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                    Box::new(cast(&r.loc(), right, &ty, true, ns)?),
                ))
            } else {
                Ok(Expression::UMoreEqual(
                    *loc,
                    Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                    Box::new(cast(&r.loc(), right, &ty, true, ns)?),
                ))
            }
        }
        pt::Expression::LessEqual(loc, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

            let ty = coerce_int(&left.ty(), &l.loc(), &right.ty(), &r.loc(), true, ns)?;

            if ty.is_signed_int() {
                Ok(Expression::SLessEqual(
                    *loc,
                    Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                    Box::new(cast(&r.loc(), right, &ty, true, ns)?),
                ))
            } else {
                Ok(Expression::ULessEqual(
                    *loc,
                    Box::new(cast(&l.loc(), left, &ty, true, ns)?),
                    Box::new(cast(&r.loc(), right, &ty, true, ns)?),
                ))
            }
        }
        pt::Expression::Equal(loc, l, r) => {
            equal(loc, l, r, file_no, contract_no, ns, symtable, is_constant)
        }

        pt::Expression::NotEqual(loc, l, r) => Ok(Expression::Not(
            *loc,
            Box::new(equal(
                loc,
                l,
                r,
                file_no,
                contract_no,
                ns,
                symtable,
                is_constant,
            )?),
        )),
        // unary expressions
        pt::Expression::Not(loc, e) => {
            let expr = expression(e, file_no, contract_no, ns, symtable, is_constant)?;

            Ok(Expression::Not(
                *loc,
                Box::new(cast(&loc, expr, &Type::Bool, true, ns)?),
            ))
        }
        pt::Expression::Complement(loc, e) => {
            let expr = expression(e, file_no, contract_no, ns, symtable, is_constant)?;

            let expr_ty = expr.ty();

            get_int_length(&expr_ty, loc, true, ns)?;

            Ok(Expression::Complement(*loc, expr_ty, Box::new(expr)))
        }
        pt::Expression::UnaryMinus(loc, e) => {
            let expr = expression(e, file_no, contract_no, ns, symtable, is_constant)?;

            let expr_type = expr.ty();

            if let Expression::NumberLiteral(_, _, n) = expr {
                bigint_to_expression(loc, &-n, ns)
            } else {
                get_int_length(&expr_type, loc, false, ns)?;

                Ok(Expression::UnaryMinus(*loc, expr_type, Box::new(expr)))
            }
        }
        pt::Expression::UnaryPlus(loc, e) => {
            let expr = expression(e, file_no, contract_no, ns, symtable, is_constant)?;
            let expr_type = expr.ty();

            get_int_length(&expr_type, loc, false, ns)?;

            Ok(expr)
        }

        pt::Expression::Ternary(loc, c, l, r) => {
            let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
            let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;
            let cond = expression(c, file_no, contract_no, ns, symtable, is_constant)?;

            let cond = cast(&c.loc(), cond, &Type::Bool, true, ns)?;

            let ty = coerce(&left.ty(), &l.loc(), &right.ty(), &r.loc(), ns)?;

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
            if is_constant {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "operator not allowed in constant context".to_string(),
                ));
                return Err(());
            };

            incr_decr(var, expr, file_no, contract_no, ns, symtable)
        }

        // assignment
        pt::Expression::Assign(loc, var, e) => {
            if is_constant {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "assignment not allowed in constant context".to_string(),
                ));
                return Err(());
            };

            assign_single(loc, var, e, file_no, contract_no, ns, symtable)
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
            if is_constant {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "assignment not allowed in constant context".to_string(),
                ));
                return Err(());
            };

            assign_expr(loc, var, expr, e, file_no, contract_no, ns, symtable)
        }
        pt::Expression::NamedFunctionCall(loc, ty, args) => {
            let marker = ns.diagnostics.len();

            // is it a struct literal
            match ns.resolve_type(file_no, contract_no, true, ty) {
                Ok(Type::Struct(n)) => {
                    return named_struct_literal(
                        loc,
                        n,
                        args,
                        file_no,
                        contract_no,
                        ns,
                        symtable,
                        is_constant,
                    );
                }
                Ok(_) => {
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        "struct or function expected".to_string(),
                    ));
                    return Err(());
                }
                _ => {}
            }

            // not a struct literal, remove those errors and try resolving as function call
            ns.diagnostics.truncate(marker);

            if is_constant {
                ns.diagnostics.push(Diagnostic::error(
                    expr.loc(),
                    "cannot call function in constant expression".to_string(),
                ));
                return Err(());
            }

            let expr = named_function_call_expr(loc, ty, args, file_no, contract_no, ns, symtable)?;

            if expr.tys().len() > 1 {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "destucturing statement needed for function that returns multiple values"
                        .to_string(),
                ));
                return Err(());
            }

            Ok(expr)
        }
        pt::Expression::New(loc, call) => {
            if is_constant {
                ns.diagnostics.push(Diagnostic::error(
                    expr.loc(),
                    "new not allowed in constant expression".to_string(),
                ));
                return Err(());
            }

            match call.as_ref() {
                pt::Expression::FunctionCall(_, ty, args) => {
                    new(loc, ty, args, file_no, contract_no, ns, symtable)
                }
                pt::Expression::NamedFunctionCall(_, ty, args) => {
                    constructor_named_args(loc, ty, args, file_no, contract_no, ns, symtable)
                }
                _ => unreachable!(),
            }
        }
        pt::Expression::Delete(loc, _) => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                "delete not allowed in expression".to_string(),
            ));
            Err(())
        }
        pt::Expression::FunctionCall(loc, ty, args) => {
            let marker = ns.diagnostics.len();

            match ns.resolve_type(file_no, contract_no, true, ty) {
                Ok(Type::Struct(n)) => {
                    return struct_literal(
                        loc,
                        n,
                        args,
                        file_no,
                        contract_no,
                        ns,
                        symtable,
                        is_constant,
                    );
                }
                Ok(to) => {
                    // Cast
                    return if args.is_empty() {
                        ns.diagnostics.push(Diagnostic::error(
                            *loc,
                            "missing argument to cast".to_string(),
                        ));
                        Err(())
                    } else if args.len() > 1 {
                        ns.diagnostics.push(Diagnostic::error(
                            *loc,
                            "too many arguments to cast".to_string(),
                        ));
                        Err(())
                    } else {
                        let expr =
                            expression(&args[0], file_no, contract_no, ns, symtable, is_constant)?;

                        cast(loc, expr, &to, false, ns)
                    };
                }
                Err(_) => {
                    ns.diagnostics.truncate(marker);
                }
            }

            if is_constant {
                ns.diagnostics.push(Diagnostic::error(
                    expr.loc(),
                    "cannot call function in constant expression".to_string(),
                ));
                return Err(());
            }

            let expr = function_call_expr(loc, ty, args, file_no, contract_no, ns, symtable)?;

            if expr.tys().len() > 1 {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "destucturing statement needed for function that returns multiple values"
                        .to_string(),
                ));
                return Err(());
            }

            Ok(expr)
        }
        pt::Expression::ArraySubscript(loc, _, None) => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                "expected expression before ‘]’ token".to_string(),
            ));

            Err(())
        }
        pt::Expression::ArraySubscript(loc, array, Some(index)) => array_subscript(
            loc,
            array,
            index,
            file_no,
            contract_no,
            ns,
            symtable,
            is_constant,
        ),
        pt::Expression::MemberAccess(loc, e, id) => {
            member_access(loc, e, id, file_no, contract_no, ns, symtable, is_constant)
        }
        pt::Expression::Or(loc, left, right) => {
            let boolty = Type::Bool;
            let l = cast(
                &loc,
                expression(left, file_no, contract_no, ns, symtable, is_constant)?,
                &boolty,
                true,
                ns,
            )?;
            let r = cast(
                &loc,
                expression(right, file_no, contract_no, ns, symtable, is_constant)?,
                &boolty,
                true,
                ns,
            )?;

            Ok(Expression::Or(*loc, Box::new(l), Box::new(r)))
        }
        pt::Expression::And(loc, left, right) => {
            let boolty = Type::Bool;
            let l = cast(
                &loc,
                expression(left, file_no, contract_no, ns, symtable, is_constant)?,
                &boolty,
                true,
                ns,
            )?;
            let r = cast(
                &loc,
                expression(right, file_no, contract_no, ns, symtable, is_constant)?,
                &boolty,
                true,
                ns,
            )?;

            Ok(Expression::And(*loc, Box::new(l), Box::new(r)))
        }
        pt::Expression::Type(loc, _) => {
            ns.diagnostics
                .push(Diagnostic::error(*loc, "type not expected".to_owned()));
            Err(())
        }
        pt::Expression::List(loc, _) => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                "lists only permitted in destructure statements".to_owned(),
            ));
            Err(())
        }
        pt::Expression::FunctionCallBlock(loc, _, _) => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                "unexpect block encountered".to_owned(),
            ));
            Err(())
        }
        pt::Expression::Unit(loc, expr, unit) => {
            let n = match expr.as_ref() {
                pt::Expression::NumberLiteral(_, n) => n,
                pt::Expression::HexNumberLiteral(loc, _) => {
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        "hexadecimal numbers cannot be used with unit denominations".to_owned(),
                    ));
                    return Err(());
                }
                _ => {
                    ns.diagnostics.push(Diagnostic::error(
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
                    ns.diagnostics.push(Diagnostic::warning(
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
            )
        }
        pt::Expression::This(loc) => match contract_no {
            Some(contract_no) => Ok(Expression::GetAddress(*loc, Type::Contract(contract_no))),
            None => {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "this not allowed outside contract".to_owned(),
                ));
                Err(())
            }
        },
    }
}

/// Resolve an new contract expression with positional arguments
fn constructor(
    loc: &pt::Loc,
    no: usize,
    args: &[pt::Expression],
    call_args: CallArgs,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<Expression, ()> {
    // The current contract cannot be constructed with new. In order to create
    // the contract, we need the code hash of the contract. Part of that code
    // will be code we're emitted here. So we end up with a crypto puzzle.
    let contract_no = match contract_no {
        Some(n) if n == no => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "new cannot construct current contract ‘{}’",
                    ns.contracts[contract_no.unwrap()].name
                ),
            ));
            return Err(());
        }
        Some(n) => n,
        None => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                "new contract not allowed in this context".to_string(),
            ));
            return Err(());
        }
    };

    if !ns.contracts[no].is_concrete() {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "cannot construct ‘{}’ of type ‘{}’",
                ns.contracts[no].name, ns.contracts[no].ty
            ),
        ));

        return Err(());
    }

    // check for circular references
    if circular_reference(no, contract_no, ns) {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "circular reference creating contract ‘{}’",
                ns.contracts[no].name
            ),
        ));
        return Err(());
    }

    if !ns.contracts[contract_no].creates.contains(&no) {
        ns.contracts[contract_no].creates.push(no);
    }

    let mut resolved_args = Vec::new();

    for arg in args {
        let expr = expression(arg, file_no, Some(contract_no), ns, symtable, false)?;

        resolved_args.push(expr);
    }

    match match_constructor_to_args(loc, resolved_args, no, ns) {
        Ok((constructor_no, cast_args)) => Ok(Expression::Constructor {
            loc: *loc,
            contract_no: no,
            constructor_no,
            args: cast_args,
            value: call_args.value,
            gas: call_args.gas,
            salt: call_args.salt,
        }),
        Err(()) => Err(()),
    }
}

/// Try and find constructor for resolved arguments
pub fn match_constructor_to_args(
    loc: &pt::Loc,
    resolved_args: Vec<Expression>,
    contract_no: usize,
    ns: &mut Namespace,
) -> Result<(Option<usize>, Vec<Expression>), ()> {
    let marker = ns.diagnostics.len();

    // constructor call
    let mut constructor_count = 0;

    for function_no in 0..ns.contracts[contract_no].functions.len() {
        if !ns.contracts[contract_no].functions[function_no].is_constructor() {
            continue;
        }

        constructor_count += 1;

        let params_len = ns.contracts[contract_no].functions[function_no]
            .params
            .len();

        if params_len != resolved_args.len() {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "constructor expects {} arguments, {} provided",
                    params_len,
                    resolved_args.len()
                ),
            ));
            continue;
        }

        let mut matches = true;
        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for (i, arg) in resolved_args.iter().enumerate() {
            match cast(
                &arg.loc(),
                arg.clone(),
                &ns.contracts[contract_no].functions[function_no].params[i]
                    .ty
                    .clone(),
                true,
                ns,
            ) {
                Ok(expr) => cast_args.push(expr),
                Err(()) => {
                    matches = false;
                    break;
                }
            }
        }

        if matches {
            return Ok((Some(function_no), cast_args));
        }
    }

    if constructor_count == 0 && resolved_args.is_empty() {
        return Ok((None, Vec::new()));
    }

    if constructor_count != 1 {
        ns.diagnostics.truncate(marker);
        ns.diagnostics.push(Diagnostic::error(
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
        .any(|n| circular_reference(*n, to, &ns))
}

/// Resolve an new contract expression with named arguments
pub fn constructor_named_args(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::NamedArgument],
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<Expression, ()> {
    let (ty, call_args, _) = collect_call_args(ty, ns)?;

    let call_args = parse_call_args(&call_args, false, file_no, contract_no, ns, symtable)?;

    let no = match ns.resolve_type(file_no, contract_no, false, ty)? {
        Type::Contract(n) => n,
        _ => {
            ns.diagnostics
                .push(Diagnostic::error(*loc, "contract expected".to_string()));
            return Err(());
        }
    };

    // The current contract cannot be constructed with new. In order to create
    // the contract, we need the code hash of the contract. Part of that code
    // will be code we're emitted here. So we end up with a crypto puzzle.
    let contract_no = match contract_no {
        Some(n) if n == no => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "new cannot construct current contract ‘{}’",
                    ns.contracts[contract_no.unwrap()].name
                ),
            ));
            return Err(());
        }
        Some(n) => n,
        None => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                "new contract not allowed in this context".to_string(),
            ));
            return Err(());
        }
    };

    if !ns.contracts[no].is_concrete() {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "cannot construct ‘{}’ of type ‘{}’",
                ns.contracts[no].name, ns.contracts[no].ty
            ),
        ));

        return Err(());
    }

    // check for circular references
    if circular_reference(no, contract_no, ns) {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "circular reference creating contract ‘{}’",
                ns.contracts[no].name
            ),
        ));
        return Err(());
    }

    if !ns.contracts[contract_no].creates.contains(&no) {
        ns.contracts[contract_no].creates.push(no);
    }

    let mut arguments = HashMap::new();

    for arg in args {
        arguments.insert(
            arg.name.name.to_string(),
            expression(&arg.expr, file_no, Some(contract_no), ns, symtable, false)?,
        );
    }

    let marker = ns.diagnostics.len();

    // constructor call
    for function_no in 0..ns.contracts[no].functions.len() {
        if !ns.contracts[no].functions[function_no].is_constructor() {
            continue;
        }

        let params_len = ns.contracts[no].functions[function_no].params.len();

        if params_len != args.len() {
            ns.diagnostics.push(Diagnostic::error(
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
            let param = ns.contracts[no].functions[function_no].params[i].clone();
            let arg = match arguments.get(&param.name) {
                Some(a) => a,
                None => {
                    matches = false;
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        format!("missing argument ‘{}’ to constructor", param.name),
                    ));
                    break;
                }
            };

            match cast(&pt::Loc(file_no, 0, 0), arg.clone(), &param.ty, true, ns) {
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
            });
        }
    }

    match ns.contracts[no]
        .functions
        .iter()
        .filter(|f| f.is_constructor())
        .count()
    {
        0 => Ok(Expression::Constructor {
            loc: *loc,
            contract_no: no,
            constructor_no: None,
            args: Vec::new(),
            value: call_args.value,
            gas: call_args.gas,
            salt: call_args.salt,
        }),
        1 => Err(()),
        _ => {
            ns.diagnostics.truncate(marker);
            ns.diagnostics.push(Diagnostic::error(
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
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
) -> Result<Expression, ()> {
    if args.is_empty() {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            "missing argument to type()".to_string(),
        ));
        return Err(());
    }

    if args.len() > 1 {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            format!("got {} arguments to type(), only one expected", args.len(),),
        ));
        return Err(());
    }

    let ty = ns.resolve_type(file_no, contract_no, false, &args[0])?;

    match (&ty, field.name.as_str()) {
        (Type::Uint(_), "min") => bigint_to_expression(loc, &BigInt::zero(), ns),
        (Type::Uint(bits), "max") => {
            let max = BigInt::one().shl(*bits as usize).sub(1);
            bigint_to_expression(loc, &max, ns)
        }
        (Type::Int(bits), "min") => {
            let min = BigInt::zero().sub(BigInt::one().shl(*bits as usize - 1));
            bigint_to_expression(loc, &min, ns)
        }
        (Type::Int(bits), "max") => {
            let max = BigInt::one().shl(*bits as usize - 1).sub(1);
            bigint_to_expression(loc, &max, ns)
        }
        (Type::Contract(n), "name") => Ok(Expression::BytesLiteral(
            *loc,
            Type::String,
            ns.contracts[*n].name.as_bytes().to_vec(),
        )),
        (Type::Contract(no), "creationCode") | (Type::Contract(no), "runtimeCode") => {
            let contract_no = match contract_no {
                Some(contract_no) => contract_no,
                None => {
                    ns.diagnostics.push(Diagnostic::error(
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
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "containing our own contract code for ‘{}’ would generate infinite size contract",
                        ns.contracts[*no].name
                    ),
                ));
                return Err(());
            }

            if circular_reference(*no, contract_no, ns) {
                ns.diagnostics.push(Diagnostic::error(
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
            ns.diagnostics.push(Diagnostic::error(
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
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<Expression, ()> {
    let (ty, call_args, call_args_loc) = collect_call_args(ty, ns)?;

    let ty = ns.resolve_type(file_no, contract_no, false, ty)?;

    match &ty {
        Type::Array(ty, dim) => {
            if dim.last().unwrap().is_some() {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "new cannot allocate fixed array type ‘{}’",
                        ty.to_string(ns)
                    ),
                ));
                return Err(());
            }

            if let Type::Contract(_) = ty.as_ref() {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    format!("new cannot construct array of ‘{}’", ty.to_string(ns)),
                ));
                return Err(());
            }
        }
        Type::String | Type::DynamicBytes => {}
        Type::Contract(n) => {
            let call_args = parse_call_args(&call_args, false, file_no, contract_no, ns, symtable)?;

            return constructor(loc, *n, args, call_args, file_no, contract_no, ns, symtable);
        }
        _ => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                format!("new cannot allocate type ‘{}’", ty.to_string(ns)),
            ));
            return Err(());
        }
    };

    if let Some(loc) = call_args_loc {
        ns.diagnostics.push(Diagnostic::error(
            loc,
            "constructor arguments not permitted for allocation".to_string(),
        ));
        return Err(());
    }

    if args.len() != 1 {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            "new dynamic array should have a single length argument".to_string(),
        ));
        return Err(());
    }
    let size_loc = args[0].loc();

    let size_expr = expression(&args[0], file_no, contract_no, ns, symtable, false)?;
    let size_ty = size_expr.ty();

    let size_width = match &size_ty {
        Type::Uint(n) => n,
        _ => {
            ns.diagnostics.push(Diagnostic::error(
                size_loc,
                format!(
                    "new size argument must be unsigned integer, not ‘{}’",
                    size_ty.to_string(ns)
                ),
            ));
            return Err(());
        }
    };

    // TODO: should we check an upper bound? Large allocations will fail anyway,
    // and ethereum solidity does not check at compile time
    let size = match size_width.cmp(&32) {
        Ordering::Greater => Expression::Trunc(size_loc, Type::Uint(32), Box::new(size_expr)),
        Ordering::Less => Expression::ZeroExt(size_loc, Type::Uint(32), Box::new(size_expr)),
        Ordering::Equal => size_expr,
    };

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
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
    is_constant: bool,
) -> Result<Expression, ()> {
    let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
    let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

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
                    &right_type.deref_any(),
                    true,
                    ns,
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
                    &left_type.deref_any(),
                    true,
                    ns,
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
                    &left_type.deref_any(),
                    true,
                    ns,
                )?)),
                StringLocation::RunTime(Box::new(cast(
                    &r.loc(),
                    right,
                    &right_type.deref_any(),
                    true,
                    ns,
                )?)),
            ));
        }
        _ => {}
    }

    let ty = coerce(&left_type, &l.loc(), &right_type, &r.loc(), ns)?;

    Ok(Expression::Equal(
        *loc,
        Box::new(cast(&l.loc(), left, &ty, true, ns)?),
        Box::new(cast(&r.loc(), right, &ty, true, ns)?),
    ))
}

/// Try string concatenation
fn addition(
    loc: &pt::Loc,
    l: &pt::Expression,
    r: &pt::Expression,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
    is_constant: bool,
) -> Result<Expression, ()> {
    let left = expression(l, file_no, contract_no, ns, symtable, is_constant)?;
    let right = expression(r, file_no, contract_no, ns, symtable, is_constant)?;

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

    let ty = coerce_int(&left_type, &l.loc(), &right_type, &r.loc(), false, ns)?;

    Ok(Expression::Add(
        *loc,
        ty.clone(),
        Box::new(cast(&l.loc(), left, &ty, true, ns)?),
        Box::new(cast(&r.loc(), right, &ty, true, ns)?),
    ))
}

/// Resolve an assignment
pub fn assign_single(
    loc: &pt::Loc,
    left: &pt::Expression,
    right: &pt::Expression,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<Expression, ()> {
    let var = expression(left, file_no, contract_no, ns, symtable, false)?;
    let var_ty = var.ty();
    let val = expression(right, file_no, contract_no, ns, symtable, false)?;

    match &var {
        Expression::ConstantVariable(loc, _, contract_no, var_no) => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "cannot assign to constant ‘{}’",
                    ns.contracts[*contract_no].variables[*var_no].name
                ),
            ));
            Err(())
        }
        Expression::StorageVariable(loc, ty, _, _) => Ok(Expression::Assign(
            *loc,
            ty.clone(),
            Box::new(var.clone()),
            Box::new(cast(&right.loc(), val, ty.deref_any(), true, ns)?),
        )),
        Expression::Variable(_, var_ty, _) => Ok(Expression::Assign(
            *loc,
            var_ty.clone(),
            Box::new(var.clone()),
            Box::new(cast(&right.loc(), val, var_ty, true, ns)?),
        )),
        _ => match &var_ty {
            Type::Ref(r_ty) | Type::StorageRef(r_ty) => Ok(Expression::Assign(
                *loc,
                var_ty.clone(),
                Box::new(var),
                Box::new(cast(&right.loc(), val, r_ty, true, ns)?),
            )),
            _ => {
                ns.diagnostics.push(Diagnostic::error(
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
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<Expression, ()> {
    let set = expression(right, file_no, contract_no, ns, symtable, false)?;
    let set_type = set.ty();

    let op = |assign: Expression, ty: &Type, ns: &mut Namespace| -> Result<Expression, ()> {
        let set = match expr {
            pt::Expression::AssignShiftLeft(_, _, _)
            | pt::Expression::AssignShiftRight(_, _, _) => {
                let left_length = get_int_length(&ty, &loc, true, ns)?;
                let right_length = get_int_length(&set_type, &left.loc(), false, ns)?;

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
            _ => cast(&right.loc(), set, &ty, true, ns)?,
        };

        Ok(match expr {
            pt::Expression::AssignAdd(_, _, _) => {
                Expression::Add(*loc, ty.clone(), Box::new(assign), Box::new(set))
            }
            pt::Expression::AssignSubtract(_, _, _) => {
                Expression::Subtract(*loc, ty.clone(), Box::new(assign), Box::new(set))
            }
            pt::Expression::AssignMultiply(_, _, _) => {
                Expression::Multiply(*loc, ty.clone(), Box::new(assign), Box::new(set))
            }
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
                if ty.is_signed_int() {
                    Expression::SDivide(*loc, ty.clone(), Box::new(assign), Box::new(set))
                } else {
                    Expression::UDivide(*loc, ty.clone(), Box::new(assign), Box::new(set))
                }
            }
            pt::Expression::AssignModulo(_, _, _) => {
                if ty.is_signed_int() {
                    Expression::SModulo(*loc, ty.clone(), Box::new(assign), Box::new(set))
                } else {
                    Expression::UModulo(*loc, ty.clone(), Box::new(assign), Box::new(set))
                }
            }
            _ => unreachable!(),
        })
    };

    let var = expression(left, file_no, contract_no, ns, symtable, false)?;
    let var_ty = var.ty();

    match &var {
        Expression::ConstantVariable(loc, _, contract_no, var_no) => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "cannot assign to constant ‘{}’",
                    ns.contracts[*contract_no].variables[*var_no].name
                ),
            ));
            Err(())
        }
        Expression::Variable(_, _, n) => {
            match var_ty {
                Type::Bytes(_) | Type::Int(_) | Type::Uint(_) => (),
                _ => {
                    ns.diagnostics.push(Diagnostic::error(
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
                Box::new(op(var, &var_ty, ns)?),
            ))
        }
        _ => match &var_ty {
            Type::Ref(r_ty) | Type::StorageRef(r_ty) => match r_ty.as_ref() {
                Type::Bytes(_) | Type::Int(_) | Type::Uint(_) => Ok(Expression::Assign(
                    *loc,
                    Type::Void,
                    Box::new(var.clone()),
                    Box::new(op(cast(loc, var, r_ty, true, ns)?, r_ty, ns)?),
                )),
                _ => {
                    ns.diagnostics.push(Diagnostic::error(
                        var.loc(),
                        format!("assigning to incorrect type {}", r_ty.to_string(ns)),
                    ));
                    Err(())
                }
            },
            _ => {
                ns.diagnostics.push(Diagnostic::error(
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
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<Expression, ()> {
    let op = |e: Expression, ty: Type| -> Expression {
        match expr {
            pt::Expression::PreIncrement(loc, _) => Expression::PreIncrement(*loc, ty, Box::new(e)),
            pt::Expression::PreDecrement(loc, _) => Expression::PreDecrement(*loc, ty, Box::new(e)),
            pt::Expression::PostIncrement(loc, _) => {
                Expression::PostIncrement(*loc, ty, Box::new(e))
            }
            pt::Expression::PostDecrement(loc, _) => {
                Expression::PostDecrement(*loc, ty, Box::new(e))
            }
            _ => unreachable!(),
        }
    };

    let var = expression(v, file_no, contract_no, ns, symtable, false)?;
    let var_ty = var.ty();

    match &var {
        Expression::ConstantVariable(loc, _, contract_no, var_no) => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "cannot assign to constant ‘{}’",
                    ns.contracts[*contract_no].variables[*var_no].name
                ),
            ));
            Err(())
        }
        Expression::Variable(_, ty, n) => {
            match ty {
                Type::Int(_) | Type::Uint(_) => (),
                _ => {
                    ns.diagnostics.push(Diagnostic::error(
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
            Type::Ref(r_ty) | Type::StorageRef(r_ty) => match r_ty.as_ref() {
                Type::Int(_) | Type::Uint(_) => Ok(op(var, r_ty.as_ref().clone())),
                _ => {
                    ns.diagnostics.push(Diagnostic::error(
                        var.loc(),
                        format!("assigning to incorrect type {}", r_ty.to_string(ns)),
                    ));
                    Err(())
                }
            },
            _ => {
                ns.diagnostics.push(Diagnostic::error(
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
    ns: &mut Namespace,
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
        if let Some(Symbol::Import(_, import_file_no)) = ns.symbols.get(&(file_no, None, name)) {
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
                ns.diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!(
                        "enum {} does not have value {}",
                        ns.enums[e].print_to_string(),
                        id.name
                    ),
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
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
    is_constant: bool,
) -> Result<Expression, ()> {
    // is it a builtin special variable like "block.timestamp"
    if let pt::Expression::Variable(namespace) = e {
        if let Some((builtin, ty)) = builtin::builtin_var(Some(&namespace.name), &id.name, ns) {
            return Ok(Expression::Builtin(*loc, vec![ty], builtin, vec![]));
        }
    }

    // is an enum value
    if let Some(expr) = enum_value(loc, e, id, file_no, contract_no, ns)? {
        return Ok(expr);
    }

    // is of the form "type(x).field", like type(c).min
    if let pt::Expression::FunctionCall(_, name, args) = e {
        if let pt::Expression::Variable(func_name) = name.as_ref() {
            if func_name.name == "type" {
                return type_name_expr(loc, args, id, file_no, contract_no, ns);
            }
        }
    }

    let expr = expression(e, file_no, contract_no, ns, symtable, is_constant)?;
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
                    Some(d) => bigint_to_expression(loc, d, ns),
                };
            }
        }
        Type::String | Type::DynamicBytes => {
            if id.name == "length" {
                return Ok(Expression::DynamicArrayLength(*loc, Box::new(expr)));
            }
        }
        Type::StorageRef(r) => match *r {
            Type::Struct(n) => {
                let mut slot = BigInt::zero();

                for field in &ns.structs[n].fields {
                    if id.name == field.name {
                        return Ok(Expression::Add(
                            *loc,
                            Type::StorageRef(Box::new(field.ty.clone())),
                            Box::new(expr),
                            Box::new(Expression::NumberLiteral(*loc, Type::Uint(256), slot)),
                        ));
                    }

                    slot += field.ty.storage_slots(ns);
                }

                ns.diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!(
                        "struct ‘{}’ does not have a field called ‘{}’",
                        ns.structs[n].name, id.name
                    ),
                ));
                return Err(());
            }
            Type::Bytes(n) => {
                if id.name == "length" {
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
                        None => Ok(Expression::StorageLoad(
                            id.loc,
                            Type::Uint(256),
                            Box::new(expr),
                        )),
                        Some(d) => bigint_to_expression(loc, d, ns),
                    };
                }
            }
            Type::DynamicBytes => {
                if id.name == "length" {
                    return Ok(Expression::StorageBytesLength(*loc, Box::new(expr)));
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
                    *loc,
                    Type::Ref(Box::new(f.ty.clone())),
                    Box::new(expr),
                    i,
                ));
            } else {
                ns.diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!(
                        "struct ‘{}’ does not have a field called ‘{}’",
                        ns.structs[n].print_to_string(),
                        id.name
                    ),
                ));
                return Err(());
            }
        }
        Type::Address(_) => {
            if id.name == "balance" {
                if ns.target == crate::Target::Substrate {
                    let mut is_this = false;

                    if let Expression::Cast(_, _, this) = &expr {
                        if let Expression::GetAddress(_, _) = this.as_ref() {
                            is_this = true;
                        }
                    }

                    if !is_this {
                        ns.diagnostics.push(Diagnostic::error(
                                    expr.loc(),
                                        "substrate can only retrieve balance of this, like ‘address(this).balance’".to_string(),
                                ));
                        return Err(());
                    }
                }

                return Ok(Expression::Balance(
                    *loc,
                    Type::Uint(ns.value_length as u16 * 8),
                    Box::new(expr),
                ));
            }
        }
        _ => (),
    }

    ns.diagnostics
        .push(Diagnostic::error(*loc, format!("‘{}’ not found", id.name)));

    Err(())
}

/// Resolve an array subscript expression
fn array_subscript(
    loc: &pt::Loc,
    array: &pt::Expression,
    index: &pt::Expression,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
    is_constant: bool,
) -> Result<Expression, ()> {
    let array_expr = expression(array, file_no, contract_no, ns, symtable, is_constant)?;
    let array_ty = array_expr.ty();

    if array_expr.ty().is_mapping() {
        return mapping_subscript(
            loc,
            array_expr,
            index,
            file_no,
            contract_no,
            ns,
            symtable,
            is_constant,
        );
    }

    let index_expr = expression(index, file_no, contract_no, ns, symtable, is_constant)?;

    match index_expr.ty() {
        Type::Uint(_) => (),
        _ => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "array subscript must be an unsigned integer, not ‘{}’",
                    index_expr.ty().to_string(ns)
                ),
            ));
            return Err(());
        }
    };

    if array_ty.is_storage_bytes() {
        return Ok(Expression::StorageBytesSubscript(
            *loc,
            Box::new(array_expr),
            Box::new(cast(&index.loc(), index_expr, &Type::Uint(32), false, ns)?),
        ));
    }

    match array_ty.deref_any() {
        Type::Bytes(_) | Type::Array(_, _) | Type::DynamicBytes => {
            if array_ty.is_contract_storage() {
                Ok(Expression::ArraySubscript(
                    *loc,
                    array_ty.storage_array_elem(),
                    Box::new(array_expr),
                    Box::new(index_expr),
                ))
            } else {
                Ok(Expression::ArraySubscript(
                    *loc,
                    array_ty.array_deref(),
                    Box::new(cast(
                        &array.loc(),
                        array_expr,
                        &array_ty.deref_any(),
                        true,
                        ns,
                    )?),
                    Box::new(index_expr),
                ))
            }
        }
        Type::String => {
            ns.diagnostics.push(Diagnostic::error(
                array.loc(),
                "array subscript is not permitted on string".to_string(),
            ));
            Err(())
        }
        _ => {
            ns.diagnostics.push(Diagnostic::error(
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
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
    is_constant: bool,
) -> Result<Expression, ()> {
    let struct_def = ns.structs[struct_no].clone();

    if args.len() != struct_def.fields.len() {
        ns.diagnostics.push(Diagnostic::error(
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
            let expr = expression(&a, file_no, contract_no, ns, symtable, is_constant)?;

            fields.push(cast(loc, expr, &struct_def.fields[i].ty, true, ns)?);
        }

        let ty = Type::Struct(struct_no);

        Ok(Expression::StructLiteral(*loc, ty, fields))
    }
}

/// Resolve a function call with positional arguments
fn function_call_pos_args(
    loc: &pt::Loc,
    id: &pt::Identifier,
    args: &[pt::Expression],
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<Expression, ()> {
    let mut resolved_args = Vec::new();

    for arg in args {
        let expr = expression(arg, file_no, contract_no, ns, symtable, false)?;

        resolved_args.push(expr);
    }

    // is it a builtin
    if builtin::is_builtin_call(None, &id.name, ns) {
        let expr = builtin::resolve_call(loc, id, resolved_args, ns)?;

        return if expr.tys().len() > 1 {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                format!("builtin function ‘{}’ returns more than one value", id.name),
            ));
            Err(())
        } else {
            Ok(expr)
        };
    }

    let mut name_matches = 0;
    let mut errors = Vec::new();

    // Try to resolve as a function call
    for (signature, (base_contract_no, function_no, _)) in
        &ns.contracts[contract_no.unwrap()].function_table
    {
        let func = &ns.contracts[*base_contract_no].functions[*function_no];

        if func.name != id.name {
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
        for (i, arg) in resolved_args.iter().enumerate() {
            match try_cast(&arg.loc(), arg.clone(), &func.params[i].ty, true, ns) {
                Ok(expr) => cast_args.push(expr),
                Err(e) => {
                    errors.push(e);
                    matches = false;
                    break;
                }
            }
        }

        if !matches {
            continue;
        }

        if Some(*base_contract_no) != contract_no && func.is_private() {
            errors.push(Diagnostic::error_with_note(
                *loc,
                "cannot call private function".to_string(),
                func.loc,
                format!("declaration of function ‘{}’", func.name),
            ));

            continue;
        }

        let returns = function_returns(func);

        return Ok(Expression::InternalFunctionCall(
            *loc,
            returns,
            signature.to_owned(),
            cast_args,
        ));
    }

    match name_matches {
        0 => {
            ns.diagnostics.push(Diagnostic::error(
                id.loc,
                format!("unknown function or type ‘{}’", id.name),
            ));
        }
        1 => ns.diagnostics.extend(errors),
        _ => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                "cannot find overloaded function which matches signature".to_string(),
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
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<Expression, ()> {
    let mut arguments = HashMap::new();

    for arg in args {
        if arguments.contains_key(&arg.name.name) {
            ns.diagnostics.push(Diagnostic::error(
                arg.name.loc,
                format!("duplicate argument with name ‘{}’", arg.name.name),
            ));
            return Err(());
        }

        arguments.insert(
            arg.name.name.to_string(),
            expression(&arg.expr, file_no, contract_no, ns, symtable, false)?,
        );
    }

    // Try to resolve as a function call
    let mut name_matches = 0;
    let mut errors = Vec::new();

    // Try to resolve as a function call
    for (signature, (base_contract_no, function_no, _)) in
        &ns.contracts[contract_no.unwrap()].function_table
    {
        let func = &ns.contracts[*base_contract_no].functions[*function_no];

        if func.name != id.name {
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
            let param = &func.params[i];

            let arg = match arguments.get(&param.name) {
                Some(a) => a,
                None => {
                    matches = false;
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "missing argument ‘{}’ to function ‘{}’",
                            param.name, id.name,
                        ),
                    ));
                    break;
                }
            };

            match try_cast(&arg.loc(), arg.clone(), &param.ty, true, ns) {
                Ok(expr) => cast_args.push(expr),
                Err(e) => {
                    errors.push(e);
                    matches = false;
                    break;
                }
            }
        }

        if !matches {
            continue;
        }

        if Some(*base_contract_no) != contract_no && func.is_private() {
            errors.push(Diagnostic::error_with_note(
                *loc,
                "cannot call private function".to_string(),
                func.loc,
                format!("declaration of function ‘{}’", func.name),
            ));

            continue;
        }

        let returns = function_returns(func);

        return Ok(Expression::InternalFunctionCall(
            *loc,
            returns,
            signature.to_owned(),
            cast_args,
        ));
    }

    match name_matches {
        0 => {
            ns.diagnostics.push(Diagnostic::error(
                id.loc,
                format!("unknown function or type ‘{}’", id.name),
            ));
        }
        1 => ns.diagnostics.extend(errors),
        _ => {
            ns.diagnostics.push(Diagnostic::error(
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
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
    is_constant: bool,
) -> Result<Expression, ()> {
    let struct_def = ns.structs[struct_no].clone();

    if args.len() != struct_def.fields.len() {
        ns.diagnostics.push(Diagnostic::error(
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
                    let expr =
                        expression(&a.expr, file_no, contract_no, ns, symtable, is_constant)?;

                    fields[i] = cast(loc, expr, &f.ty, true, ns)?;
                }
                None => {
                    ns.diagnostics.push(Diagnostic::error(
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
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<Expression, ()> {
    if let pt::Expression::Variable(namespace) = var {
        if builtin::is_builtin_call(Some(&namespace.name), &func.name, ns) {
            if let Some(loc) = call_args_loc {
                ns.diagnostics.push(Diagnostic::error(
                    loc,
                    "call arguments not allowed on builtins".to_string(),
                ));
                return Err(());
            }

            return builtin::resolve_method_call(
                loc,
                file_no,
                namespace,
                func,
                args,
                contract_no,
                ns,
                symtable,
            );
        }
    }

    let var_expr = expression(var, file_no, contract_no, ns, symtable, false)?;
    let var_ty = var_expr.ty();

    if let Type::StorageRef(ty) = &var_ty {
        match ty.as_ref() {
            Type::Array(_, dim) => {
                if let Some(loc) = call_args_loc {
                    ns.diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on arrays".to_string(),
                    ));
                    return Err(());
                }

                if func.name == "push" {
                    if dim.last().unwrap().is_some() {
                        ns.diagnostics.push(Diagnostic::error(
                            func.loc,
                            "method ‘push()’ not allowed on fixed length array".to_string(),
                        ));
                        return Err(());
                    }

                    let elem_ty = ty.array_elem();
                    let mut builtin_args = vec![var_expr];

                    let ret_ty = match args.len() {
                        1 => {
                            let expr =
                                expression(&args[0], file_no, contract_no, ns, symtable, false)?;

                            builtin_args.push(cast(&args[0].loc(), expr, &elem_ty, true, ns)?);

                            Type::Void
                        }
                        0 => {
                            if elem_ty.is_reference_type() {
                                Type::StorageRef(Box::new(elem_ty))
                            } else {
                                elem_ty
                            }
                        }
                        _ => {
                            ns.diagnostics.push(Diagnostic::error(
                                func.loc,
                                "method ‘push()’ takes at most 1 argument".to_string(),
                            ));
                            return Err(());
                        }
                    };

                    return Ok(Expression::Builtin(
                        *loc,
                        vec![ret_ty],
                        Builtin::ArrayPush,
                        builtin_args,
                    ));
                }
                if func.name == "pop" {
                    if dim.last().unwrap().is_some() {
                        ns.diagnostics.push(Diagnostic::error(
                            func.loc,
                            "method ‘pop()’ not allowed on fixed length array".to_string(),
                        ));

                        return Err(());
                    }

                    if !args.is_empty() {
                        ns.diagnostics.push(Diagnostic::error(
                            func.loc,
                            "method ‘pop()’ does not take any arguments".to_string(),
                        ));
                        return Err(());
                    }

                    let storage_elem = ty.storage_array_elem();
                    let elem_ty = storage_elem.deref_any();

                    return Ok(Expression::Builtin(
                        *loc,
                        vec![elem_ty.clone()],
                        Builtin::ArrayPop,
                        vec![var_expr],
                    ));
                }
            }
            Type::DynamicBytes => {
                if let Some(loc) = call_args_loc {
                    ns.diagnostics.push(Diagnostic::error(
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
                            let expr =
                                expression(&args[0], file_no, contract_no, ns, symtable, false)?;

                            builtin_args.push(cast(&args[0].loc(), expr, &elem_ty, true, ns)?);

                            Type::Void
                        }
                        0 => elem_ty,
                        _ => {
                            ns.diagnostics.push(Diagnostic::error(
                                func.loc,
                                "method ‘push()’ takes at most 1 argument".to_string(),
                            ));
                            return Err(());
                        }
                    };

                    return Ok(Expression::Builtin(
                        *loc,
                        vec![ret_ty],
                        Builtin::BytesPush,
                        builtin_args,
                    ));
                }

                if func.name == "pop" {
                    if !args.is_empty() {
                        ns.diagnostics.push(Diagnostic::error(
                            func.loc,
                            "method ‘pop()’ does not take any arguments".to_string(),
                        ));
                        return Err(());
                    }

                    return Ok(Expression::Builtin(
                        *loc,
                        vec![Type::Bytes(1)],
                        Builtin::BytesPop,
                        vec![var_expr],
                    ));
                }
            }
            _ => {}
        }
    }

    if matches!(var_ty, Type::Array(..) | Type::DynamicBytes) {
        if func.name == "push" {
            let elem_ty = match &var_ty {
                Type::Array(ty, _) => ty,
                Type::DynamicBytes => &Type::Uint(8),
                _ => unreachable!(),
            };
            let val = match args.len() {
                0 => elem_ty.default(ns),
                1 => {
                    let val_expr = expression(&args[0], file_no, contract_no, ns, symtable, false)?;

                    cast(&args[0].loc(), val_expr, elem_ty, true, ns)?
                }
                _ => {
                    ns.diagnostics.push(Diagnostic::error(
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
            if !args.is_empty() {
                ns.diagnostics.push(Diagnostic::error(
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

    if let Type::Contract(contract_no) = &var_ty.deref_any() {
        let call_args =
            parse_call_args(call_args, true, file_no, Some(*contract_no), ns, symtable)?;

        let mut resolved_args = Vec::new();

        for arg in args {
            let expr = expression(arg, file_no, Some(*contract_no), ns, symtable, false)?;
            resolved_args.push(Box::new(expr));
        }

        let marker = ns.diagnostics.len();
        let mut name_match = 0;

        for function_no in 0..ns.contracts[*contract_no].functions.len() {
            if func.name != ns.contracts[*contract_no].functions[function_no].name {
                continue;
            }

            name_match += 1;

            let params_len = ns.contracts[*contract_no].functions[function_no]
                .params
                .len();

            if params_len != args.len() {
                ns.diagnostics.push(Diagnostic::error(
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
            for (i, arg) in resolved_args.iter().enumerate() {
                match cast(
                    &arg.loc(),
                    *arg.clone(),
                    &ns.contracts[*contract_no].functions[function_no].params[i]
                        .ty
                        .clone(),
                    true,
                    ns,
                ) {
                    Ok(expr) => cast_args.push(expr),
                    Err(()) => {
                        matches = false;
                        break;
                    }
                }
            }
            if matches {
                ns.diagnostics.truncate(marker);

                if !ns.contracts[*contract_no].functions[function_no].is_public() {
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        format!("function ‘{}’ is not ‘public’ or ‘extern’", func.name),
                    ));
                    return Err(());
                }

                let value = if let Some(value) = call_args.value {
                    if !value.const_zero(Some(*contract_no), ns)
                        && !ns.contracts[*contract_no].functions[function_no].is_payable()
                    {
                        ns.diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "sending value to function ‘{}’ which is not payable",
                                func.name
                            ),
                        ));
                        return Err(());
                    }

                    value
                } else {
                    Box::new(Expression::NumberLiteral(
                        pt::Loc(0, 0, 0),
                        Type::Uint(ns.value_length as u16 * 8),
                        BigInt::zero(),
                    ))
                };

                let returns = function_returns(&ns.contracts[*contract_no].functions[function_no]);

                return Ok(Expression::ExternalFunctionCall {
                    loc: *loc,
                    contract_no: *contract_no,
                    function_no,
                    returns,
                    address: Box::new(cast(
                        &var.loc(),
                        var_expr,
                        &Type::Contract(*contract_no),
                        true,
                        ns,
                    )?),
                    args: cast_args,
                    value,
                    gas: call_args.gas,
                });
            }
        }

        if name_match != 1 {
            ns.diagnostics.truncate(marker);
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                "cannot find overloaded function which matches signature".to_string(),
            ));
        }

        return Err(());
    }

    if let Type::Address(true) = &var_ty.deref_any() {
        if func.name == "transfer" || func.name == "send" {
            if args.len() != 1 {
                ns.diagnostics.push(Diagnostic::error(
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
                ns.diagnostics.push(Diagnostic::error(
                    loc,
                    format!("call arguments not allowed on ‘{}’", func.name),
                ));
                return Err(());
            }

            let expr = expression(&args[0], file_no, contract_no, ns, symtable, false)?;

            let value = cast(
                &args[0].loc(),
                expr,
                &Type::Uint(ns.value_length as u16 * 8),
                true,
                ns,
            )?;

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
            let call_args = parse_call_args(call_args, true, file_no, contract_no, ns, symtable)?;

            if args.len() != 1 {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "‘{}’ expects 1 argument, {} provided",
                        func.name,
                        args.len()
                    ),
                ));

                return Err(());
            }

            let expr = expression(&args[0], file_no, contract_no, ns, symtable, false)?;

            let args = cast(&args[0].loc(), expr, &Type::DynamicBytes, true, ns)?;

            let value = call_args.value.unwrap_or_else(|| {
                Box::new(Expression::NumberLiteral(
                    pt::Loc(0, 0, 0),
                    Type::Uint(ns.value_length as u16 * 8),
                    BigInt::zero(),
                ))
            });

            return Ok(Expression::ExternalFunctionCallRaw {
                loc: *loc,
                ty,
                args: Box::new(args),
                address: Box::new(var_expr),
                gas: call_args.gas,
                value,
            });
        }
    }
    ns.diagnostics.push(Diagnostic::error(
        func.loc,
        format!("method ‘{}’ does not exist", func.name),
    ));

    Err(())
}

fn method_call_named_args(
    loc: &pt::Loc,
    var: &pt::Expression,
    func_name: &pt::Identifier,
    args: &[pt::NamedArgument],
    call_args: &[&pt::NamedArgument],
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<Expression, ()> {
    let var_expr = expression(var, file_no, contract_no, ns, symtable, false)?;
    let var_ty = var_expr.ty();

    if let Type::Contract(external_contract_no) = &var_ty.deref_any() {
        let call_args = parse_call_args(&call_args, true, file_no, contract_no, ns, symtable)?;

        let mut arguments = HashMap::new();

        for arg in args {
            if arguments.contains_key(&arg.name.name) {
                ns.diagnostics.push(Diagnostic::error(
                    arg.name.loc,
                    format!("duplicate argument with name ‘{}’", arg.name.name),
                ));
                return Err(());
            }

            arguments.insert(
                arg.name.name.to_string(),
                expression(&arg.expr, file_no, contract_no, ns, symtable, false)?,
            );
        }

        let marker = ns.diagnostics.len();
        let mut name_match = 0;

        // function call
        for function_no in 0..ns.contracts[*external_contract_no].functions.len() {
            if ns.contracts[*external_contract_no].functions[function_no].name != func_name.name {
                continue;
            }

            let params_len = ns.contracts[*external_contract_no].functions[function_no]
                .params
                .len();

            name_match += 1;

            if params_len != args.len() {
                ns.diagnostics.push(Diagnostic::error(
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
                let param =
                    ns.contracts[*external_contract_no].functions[function_no].params[i].clone();

                let arg = match arguments.get(&param.name) {
                    Some(a) => a,
                    None => {
                        matches = false;
                        ns.diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "missing argument ‘{}’ to function ‘{}’",
                                param.name, func_name.name,
                            ),
                        ));
                        break;
                    }
                };
                match cast(&pt::Loc(0, 0, 0), arg.clone(), &param.ty, true, ns) {
                    Ok(expr) => cast_args.push(expr),
                    Err(()) => {
                        matches = false;
                        break;
                    }
                }
            }

            if matches {
                if !ns.contracts[*external_contract_no].functions[function_no].is_public() {
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        format!("function ‘{}’ is not ‘public’ or ‘extern’", func_name.name),
                    ));
                    return Err(());
                }

                let value = if let Some(value) = call_args.value {
                    if !value.const_zero(contract_no, ns)
                        && !ns.contracts[*external_contract_no].functions[function_no].is_payable()
                    {
                        ns.diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "sending value to function ‘{}’ which is not payable",
                                func_name.name
                            ),
                        ));
                        return Err(());
                    }

                    value
                } else {
                    Box::new(Expression::NumberLiteral(
                        pt::Loc(0, 0, 0),
                        Type::Uint(ns.value_length as u16 * 8),
                        BigInt::zero(),
                    ))
                };

                let returns =
                    function_returns(&ns.contracts[*external_contract_no].functions[function_no]);

                return Ok(Expression::ExternalFunctionCall {
                    loc: *loc,
                    contract_no: *external_contract_no,
                    function_no,
                    returns,
                    address: Box::new(cast(
                        &var.loc(),
                        var_expr,
                        &Type::Contract(*external_contract_no),
                        true,
                        ns,
                    )?),
                    args: cast_args,
                    value,
                    gas: call_args.gas,
                });
            }
        }

        match name_match {
            0 => {
                ns.diagnostics.push(Diagnostic::error(
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
                ns.diagnostics.truncate(marker);
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "cannot find overloaded function which matches signature".to_string(),
                ));
            }
        }
        return Err(());
    }

    ns.diagnostics.push(Diagnostic::error(
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
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
    is_constant: bool,
) -> Result<Expression, ()> {
    let mut dims = Box::new(Vec::new());
    let mut flattened = Vec::new();

    check_subarrays(exprs, &mut Some(&mut dims), &mut flattened, ns)?;

    if flattened.is_empty() {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            "array requires at least one element".to_string(),
        ));
        return Err(());
    }

    let mut flattened = flattened.iter();

    // We follow the solidity scheme were everthing gets implicitly converted to the
    // type of the first element
    let first = expression(
        flattened.next().unwrap(),
        file_no,
        contract_no,
        ns,
        symtable,
        is_constant,
    )?;

    let ty = first.ty();
    let mut exprs = vec![first];

    for e in flattened {
        let mut other = expression(e, file_no, contract_no, ns, symtable, is_constant)?;

        if other.ty() != ty {
            other = cast(&e.loc(), other, &ty, true, ns)?;
        }

        exprs.push(other);
    }

    let aty = Type::Array(
        Box::new(ty),
        dims.iter()
            .map(|n| Some(BigInt::from_u32(*n).unwrap()))
            .collect::<Vec<Option<BigInt>>>(),
    );

    if is_constant {
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
    ns: &mut Namespace,
) -> Result<(), ()> {
    if let Some(pt::Expression::ArrayLiteral(_, first)) = exprs.get(0) {
        // ensure all elements are array literals of the same length
        check_subarrays(first, dims, flatten, ns)?;

        for (i, e) in exprs.iter().enumerate().skip(1) {
            if let pt::Expression::ArrayLiteral(_, other) = e {
                if other.len() != first.len() {
                    ns.diagnostics.push(Diagnostic::error(
                        e.loc(),
                        format!(
                            "array elements should be identical, sub array {} has {} elements rather than {}", i + 1, other.len(), first.len()
                        ),
                    ));
                    return Err(());
                }
                check_subarrays(other, &mut None, flatten, ns)?;
            } else {
                ns.diagnostics.push(Diagnostic::error(
                    e.loc(),
                    format!("array element {} should also be an array", i + 1),
                ));
                return Err(());
            }
        }
    } else {
        for (i, e) in exprs.iter().enumerate().skip(1) {
            if let pt::Expression::ArrayLiteral(loc, _) = e {
                ns.diagnostics.push(Diagnostic::error(
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
    ns: &mut Namespace,
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
            pt::Statement::Block(_, s) if s.is_empty() => {
                // {}
                ns.diagnostics.push(Diagnostic::error(
                    block.loc(),
                    "missing call arguments".to_string(),
                ));
                return Err(());
            }
            _ => {
                ns.diagnostics.push(Diagnostic::error(
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
    gas: Box<Expression>,
    salt: Option<Box<Expression>>,
    value: Option<Box<Expression>>,
}

/// Parse call arguments for external calls
fn parse_call_args(
    call_args: &[&pt::NamedArgument],
    external_call: bool,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<CallArgs, ()> {
    let mut args: HashMap<&String, &pt::NamedArgument> = HashMap::new();

    for arg in call_args {
        if let Some(prev) = args.get(&arg.name.name) {
            ns.diagnostics.push(Diagnostic::error_with_note(
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
        gas: Box::new(Expression::NumberLiteral(
            pt::Loc(0, 0, 0),
            Type::Uint(64),
            BigInt::zero(),
        )),
        value: None,
        salt: None,
    };

    for arg in args.values() {
        match arg.name.name.as_str() {
            "value" => {
                let expr = expression(&arg.expr, file_no, contract_no, ns, symtable, false)?;

                let ty = Type::Uint(ns.value_length as u16 * 8);

                res.value = Some(Box::new(cast(&arg.expr.loc(), expr, &ty, true, ns)?));
            }
            "gas" => {
                let expr = expression(&arg.expr, file_no, contract_no, ns, symtable, false)?;

                let ty = Type::Uint(64);

                res.gas = Box::new(cast(&arg.expr.loc(), expr, &ty, true, ns)?);
            }
            "salt" => {
                if external_call {
                    ns.diagnostics.push(Diagnostic::error(
                        arg.loc,
                        "‘salt’ not valid for external calls".to_string(),
                    ));
                    return Err(());
                }

                let expr = expression(&arg.expr, file_no, contract_no, ns, symtable, false)?;

                let ty = Type::Uint(256);

                res.salt = Some(Box::new(cast(&arg.expr.loc(), expr, &ty, true, ns)?));
            }
            _ => {
                ns.diagnostics.push(Diagnostic::error(
                    arg.loc,
                    format!("‘{}’ not a valid call parameter", arg.name.name),
                ));
                return Err(());
            }
        }
    }

    Ok(res)
}

/// Resolve function call
pub fn function_call_expr(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::Expression],
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<Expression, ()> {
    let (ty, call_args, call_args_loc) = collect_call_args(ty, ns)?;

    match ty {
        pt::Expression::MemberAccess(_, member, func) => method_call_pos_args(
            loc,
            member,
            func,
            args,
            &call_args,
            call_args_loc,
            file_no,
            contract_no,
            ns,
            symtable,
        ),
        pt::Expression::Variable(id) => {
            if let Some(loc) = call_args_loc {
                ns.diagnostics.push(Diagnostic::error(
                    loc,
                    "call arguments not permitted for internal calls".to_string(),
                ));
                return Err(());
            }

            function_call_pos_args(loc, &id, args, file_no, contract_no, ns, symtable)
        }
        pt::Expression::ArraySubscript(_, _, _) => {
            ns.diagnostics.push(Diagnostic::error(
                ty.loc(),
                "unexpected array type".to_string(),
            ));
            Err(())
        }
        _ => {
            ns.diagnostics.push(Diagnostic::error(
                ty.loc(),
                "expression not expected here".to_string(),
            ));
            Err(())
        }
    }
}

/// Resolve function call expression with named arguments
pub fn named_function_call_expr(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::NamedArgument],
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<Expression, ()> {
    let (ty, call_args, call_args_loc) = collect_call_args(ty, ns)?;

    match ty {
        pt::Expression::MemberAccess(_, member, func) => method_call_named_args(
            loc,
            member,
            func,
            args,
            &call_args,
            file_no,
            contract_no,
            ns,
            symtable,
        ),
        pt::Expression::Variable(id) => {
            if let Some(loc) = call_args_loc {
                ns.diagnostics.push(Diagnostic::error(
                    loc,
                    "call arguments not permitted for internal calls".to_string(),
                ));
                return Err(());
            }

            function_call_with_named_args(loc, &id, args, file_no, contract_no, ns, symtable)
        }
        pt::Expression::ArraySubscript(_, _, _) => {
            ns.diagnostics.push(Diagnostic::error(
                ty.loc(),
                "unexpected array type".to_string(),
            ));
            Err(())
        }
        _ => {
            ns.diagnostics.push(Diagnostic::error(
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

/// Calculate storage subscript
fn mapping_subscript(
    loc: &pt::Loc,
    mapping: Expression,
    index: &pt::Expression,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
    is_constant: bool,
) -> Result<Expression, ()> {
    let ty = mapping.ty();

    let (key_ty, value_ty) = match ty.deref_any() {
        Type::Mapping(k, v) => (k, v),
        _ => unreachable!(),
    };

    let index_expr = cast(
        &index.loc(),
        expression(index, file_no, contract_no, ns, symtable, is_constant)?,
        key_ty,
        true,
        ns,
    )?;

    Ok(Expression::ArraySubscript(
        *loc,
        Type::StorageRef(value_ty.clone()),
        Box::new(mapping),
        Box::new(index_expr),
    ))
}
