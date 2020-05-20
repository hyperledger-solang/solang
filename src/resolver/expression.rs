use num_bigint::BigInt;
use num_bigint::Sign;
use num_traits::FromPrimitive;
use num_traits::Num;
use num_traits::One;
use num_traits::ToPrimitive;
use num_traits::Zero;
use std::cmp;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::Mul;
use std::ops::Shl;
use std::ops::Sub;

use hex;
use output;
use output::Output;
use parser::ast;
use parser::ast::Loc;
use resolver;
use resolver::address::to_hexstr_eip55;
use resolver::cfg::{resolve_var_decl_ty, ControlFlowGraph, Instr, Storage, Vartable};
use resolver::eval::eval_number_expression;
use resolver::storage::{
    array_offset, array_pop, array_push, bytes_pop, bytes_push, delete, mapping_subscript,
};

#[derive(PartialEq, Clone, Debug)]
pub enum Expression {
    BoolLiteral(Loc, bool),
    BytesLiteral(Loc, Vec<u8>),
    CodeLiteral(Loc, usize, bool),
    NumberLiteral(Loc, u16, BigInt),
    StructLiteral(Loc, resolver::Type, Vec<Expression>),
    ArrayLiteral(Loc, resolver::Type, Vec<u32>, Vec<Expression>),
    ConstArrayLiteral(Loc, Vec<u32>, Vec<Expression>),
    Add(Loc, Box<Expression>, Box<Expression>),
    Subtract(Loc, Box<Expression>, Box<Expression>),
    Multiply(Loc, Box<Expression>, Box<Expression>),
    UDivide(Loc, Box<Expression>, Box<Expression>),
    SDivide(Loc, Box<Expression>, Box<Expression>),
    UModulo(Loc, Box<Expression>, Box<Expression>),
    SModulo(Loc, Box<Expression>, Box<Expression>),
    Power(Loc, Box<Expression>, Box<Expression>),
    BitwiseOr(Loc, Box<Expression>, Box<Expression>),
    BitwiseAnd(Loc, Box<Expression>, Box<Expression>),
    BitwiseXor(Loc, Box<Expression>, Box<Expression>),
    ShiftLeft(Loc, Box<Expression>, Box<Expression>),
    ShiftRight(Loc, Box<Expression>, Box<Expression>, bool),
    Variable(Loc, usize),
    Load(Loc, Box<Expression>),
    StorageLoad(Loc, resolver::Type, Box<Expression>),
    ZeroExt(Loc, resolver::Type, Box<Expression>),
    SignExt(Loc, resolver::Type, Box<Expression>),
    Trunc(Loc, resolver::Type, Box<Expression>),

    UMore(Loc, Box<Expression>, Box<Expression>),
    ULess(Loc, Box<Expression>, Box<Expression>),
    UMoreEqual(Loc, Box<Expression>, Box<Expression>),
    ULessEqual(Loc, Box<Expression>, Box<Expression>),
    SMore(Loc, Box<Expression>, Box<Expression>),
    SLess(Loc, Box<Expression>, Box<Expression>),
    SMoreEqual(Loc, Box<Expression>, Box<Expression>),
    SLessEqual(Loc, Box<Expression>, Box<Expression>),
    Equal(Loc, Box<Expression>, Box<Expression>),
    NotEqual(Loc, Box<Expression>, Box<Expression>),

    Not(Loc, Box<Expression>),
    Complement(Loc, Box<Expression>),
    UnaryMinus(Loc, Box<Expression>),

    Ternary(Loc, Box<Expression>, Box<Expression>, Box<Expression>),
    ArraySubscript(Loc, Box<Expression>, Box<Expression>),
    StructMember(Loc, Box<Expression>, usize),

    AllocDynamicArray(Loc, resolver::Type, Box<Expression>, Option<Vec<u8>>),
    DynamicArrayLength(Loc, Box<Expression>),
    DynamicArraySubscript(Loc, Box<Expression>, resolver::Type, Box<Expression>),
    StorageBytesSubscript(Loc, Box<Expression>, Box<Expression>),
    StorageBytesPush(Loc, Box<Expression>, Box<Expression>),
    StorageBytesPop(Loc, Box<Expression>),
    StorageBytesLength(Loc, Box<Expression>),
    StringCompare(Loc, StringLocation, StringLocation),
    StringConcat(Loc, StringLocation, StringLocation),

    Or(Loc, Box<Expression>, Box<Expression>),
    And(Loc, Box<Expression>, Box<Expression>),
    LocalFunctionCall(Loc, usize, Vec<Expression>),
    ExternalFunctionCall(Loc, usize, usize, Box<Expression>, Vec<Expression>),
    Constructor(Loc, usize, usize, Vec<Expression>),

    Keccak256(Loc, Vec<(Expression, resolver::Type)>),

    ReturnData(Loc),
    Poison,
    Unreachable,
}

#[derive(PartialEq, Clone, Debug)]
pub enum StringLocation {
    CompileTime(Vec<u8>),
    RunTime(Box<Expression>),
}

impl Expression {
    /// Return the location for this expression
    pub fn loc(&self) -> Loc {
        match self {
            Expression::BoolLiteral(loc, _)
            | Expression::BytesLiteral(loc, _)
            | Expression::CodeLiteral(loc, _, _)
            | Expression::NumberLiteral(loc, _, _)
            | Expression::StructLiteral(loc, _, _)
            | Expression::ArrayLiteral(loc, _, _, _)
            | Expression::ConstArrayLiteral(loc, _, _)
            | Expression::Add(loc, _, _)
            | Expression::Subtract(loc, _, _)
            | Expression::Multiply(loc, _, _)
            | Expression::UDivide(loc, _, _)
            | Expression::SDivide(loc, _, _)
            | Expression::UModulo(loc, _, _)
            | Expression::SModulo(loc, _, _)
            | Expression::Power(loc, _, _)
            | Expression::BitwiseOr(loc, _, _)
            | Expression::BitwiseAnd(loc, _, _)
            | Expression::BitwiseXor(loc, _, _)
            | Expression::ShiftLeft(loc, _, _)
            | Expression::ShiftRight(loc, _, _, _)
            | Expression::Variable(loc, _)
            | Expression::Load(loc, _)
            | Expression::StorageLoad(loc, _, _)
            | Expression::ZeroExt(loc, _, _)
            | Expression::SignExt(loc, _, _)
            | Expression::Trunc(loc, _, _)
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
            | Expression::Complement(loc, _)
            | Expression::UnaryMinus(loc, _)
            | Expression::Ternary(loc, _, _, _)
            | Expression::ArraySubscript(loc, _, _)
            | Expression::StructMember(loc, _, _)
            | Expression::Or(loc, _, _)
            | Expression::AllocDynamicArray(loc, _, _, _)
            | Expression::DynamicArrayLength(loc, _)
            | Expression::DynamicArraySubscript(loc, _, _, _)
            | Expression::StorageBytesSubscript(loc, _, _)
            | Expression::StorageBytesPush(loc, _, _)
            | Expression::StorageBytesPop(loc, _)
            | Expression::StorageBytesLength(loc, _)
            | Expression::StringCompare(loc, _, _)
            | Expression::StringConcat(loc, _, _)
            | Expression::Keccak256(loc, _)
            | Expression::ReturnData(loc)
            | Expression::LocalFunctionCall(loc, _, _)
            | Expression::ExternalFunctionCall(loc, _, _, _, _)
            | Expression::Constructor(loc, _, _, _)
            | Expression::And(loc, _, _) => *loc,
            Expression::Poison | Expression::Unreachable => unreachable!(),
        }
    }
    /// Returns true if the Expression may load from contract storage using StorageLoad
    pub fn reads_contract_storage(&self) -> bool {
        match self {
            Expression::StorageLoad(_, _, _) => true,
            Expression::BoolLiteral(_, _)
            | Expression::BytesLiteral(_, _)
            | Expression::CodeLiteral(_, _, _)
            | Expression::NumberLiteral(_, _, _) => false,
            Expression::StructLiteral(_, _, exprs) => {
                exprs.iter().any(|e| e.reads_contract_storage())
            }
            Expression::ArrayLiteral(_, _, _, exprs) => {
                exprs.iter().any(|e| e.reads_contract_storage())
            }
            Expression::ConstArrayLiteral(_, _, _) => false,
            Expression::Add(_, l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::Subtract(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::Multiply(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::UDivide(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::SDivide(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::UModulo(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::SModulo(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }

            Expression::Power(_, l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::BitwiseOr(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::BitwiseAnd(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::BitwiseXor(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::ShiftLeft(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::ShiftRight(_, l, r, _) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }

            Expression::Variable(_, _) | Expression::Load(_, _) => false,
            Expression::ZeroExt(_, _, e) => e.reads_contract_storage(),
            Expression::SignExt(_, _, e) => e.reads_contract_storage(),
            Expression::Trunc(_, _, e) => e.reads_contract_storage(),

            Expression::UMore(_, l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::ULess(_, l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::UMoreEqual(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::ULessEqual(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::SMore(_, l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::SLess(_, l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::SLessEqual(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::SMoreEqual(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::Equal(_, l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::NotEqual(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }

            Expression::Not(_, e) => e.reads_contract_storage(),
            Expression::Complement(_, e) => e.reads_contract_storage(),
            Expression::UnaryMinus(_, e) => e.reads_contract_storage(),

            Expression::Ternary(_, c, l, r) => {
                c.reads_contract_storage()
                    || l.reads_contract_storage()
                    || r.reads_contract_storage()
            }
            Expression::DynamicArraySubscript(_, l, _, r) | Expression::ArraySubscript(_, l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::DynamicArrayLength(_, e) | Expression::AllocDynamicArray(_, _, e, _) => {
                e.reads_contract_storage()
            }
            Expression::StorageBytesSubscript(_, _, _)
            | Expression::StorageBytesPush(_, _, _)
            | Expression::StorageBytesPop(_, _)
            | Expression::StorageBytesLength(_, _) => true,
            Expression::StructMember(_, s, _) => s.reads_contract_storage(),
            Expression::LocalFunctionCall(_, _, args)
            | Expression::Constructor(_, _, _, args)
            | Expression::ExternalFunctionCall(_, _, _, _, args) => {
                args.iter().any(|a| a.reads_contract_storage())
            }
            Expression::Keccak256(_, e) => e.iter().any(|e| e.0.reads_contract_storage()),
            Expression::And(_, l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::Or(_, l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::StringConcat(_, l, r) | Expression::StringCompare(_, l, r) => {
                if let StringLocation::RunTime(e) = l {
                    if !e.reads_contract_storage() {
                        return false;
                    }
                }
                if let StringLocation::RunTime(e) = r {
                    return e.reads_contract_storage();
                }
                false
            }
            Expression::ReturnData(_) => false,
            Expression::Poison => false,
            Expression::Unreachable => false,
        }
    }
}

/// Unescape a string literal
fn unescape(literal: &str, start: usize, errors: &mut Vec<output::Output>) -> String {
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
                        errors.push(Output::error(
                            ast::Loc(start + i, start + i + 4),
                            format!("\\x{:02x} is not a valid unicode character", ch),
                        ));
                    }
                },
                Err(offset) => {
                    errors.push(Output::error(
                        ast::Loc(start + i, start + std::cmp::min(literal.len(), offset)),
                        "\\x escape should be followed by two hex digits".to_string(),
                    ));
                }
            },
            Some((i, 'u')) => match get_digits(&mut indeces, 4) {
                Ok(ch) => match std::char::from_u32(ch) {
                    Some(ch) => s.push(ch),
                    None => {
                        errors.push(Output::error(
                            ast::Loc(start + i, start + i + 6),
                            format!("\\u{:04x} is not a valid unicode character", ch),
                        ));
                    }
                },
                Err(offset) => {
                    errors.push(Output::error(
                        ast::Loc(start + i, start + std::cmp::min(literal.len(), offset)),
                        "\\u escape should be followed by four hex digits".to_string(),
                    ));
                }
            },
            Some((i, ch)) => {
                errors.push(Output::error(
                    ast::Loc(start + i, start + i + ch.len_utf8()),
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
    l: &resolver::Type,
    l_loc: &ast::Loc,
    r: &resolver::Type,
    r_loc: &ast::Loc,
    ns: &resolver::Namespace,
    errors: &mut Vec<output::Output>,
) -> Result<resolver::Type, ()> {
    let l = match l {
        resolver::Type::Ref(ty) => ty,
        resolver::Type::StorageRef(ty) => ty,
        _ => l,
    };
    let r = match r {
        resolver::Type::Ref(ty) => ty,
        resolver::Type::StorageRef(ty) => ty,
        _ => r,
    };

    if *l == *r {
        return Ok(l.clone());
    }

    coerce_int(l, l_loc, r, r_loc, true, ns, errors)
}

fn get_int_length(
    l: &resolver::Type,
    l_loc: &ast::Loc,
    allow_bytes: bool,
    ns: &resolver::Namespace,
    errors: &mut Vec<output::Output>,
) -> Result<(u16, bool), ()> {
    match l {
        resolver::Type::Uint(n) => Ok((*n, false)),
        resolver::Type::Int(n) => Ok((*n, true)),
        resolver::Type::Bytes(n) if allow_bytes => Ok((*n as u16 * 8, false)),
        resolver::Type::Enum(n) => {
            errors.push(Output::error(
                *l_loc,
                format!("type enum {} not allowed", ns.enums[*n].print_to_string(),),
            ));
            Err(())
        }
        resolver::Type::Struct(n) => {
            errors.push(Output::error(
                *l_loc,
                format!(
                    "type struct {} not allowed",
                    ns.structs[*n].print_to_string()
                ),
            ));
            Err(())
        }
        resolver::Type::Array(_, _) => {
            errors.push(Output::error(
                *l_loc,
                format!("type array {} not allowed", l.to_string(ns)),
            ));
            Err(())
        }
        resolver::Type::Ref(n) => get_int_length(n, l_loc, allow_bytes, ns, errors),
        resolver::Type::StorageRef(n) => get_int_length(n, l_loc, allow_bytes, ns, errors),
        resolver::Type::Undef => {
            unreachable!();
        }
        _ => {
            errors.push(Output::error(
                *l_loc,
                format!("expression of type {} not allowed", l.to_string(ns)),
            ));
            Err(())
        }
    }
}

fn coerce_int(
    l: &resolver::Type,
    l_loc: &ast::Loc,
    r: &resolver::Type,
    r_loc: &ast::Loc,
    allow_bytes: bool,
    ns: &resolver::Namespace,
    errors: &mut Vec<output::Output>,
) -> Result<resolver::Type, ()> {
    let l = match l {
        resolver::Type::Ref(ty) => ty,
        resolver::Type::StorageRef(ty) => ty,
        _ => l,
    };
    let r = match r {
        resolver::Type::Ref(ty) => ty,
        resolver::Type::StorageRef(ty) => ty,
        _ => r,
    };

    match (l, r) {
        (resolver::Type::Bytes(left_length), resolver::Type::Bytes(right_length))
            if allow_bytes =>
        {
            return Ok(resolver::Type::Bytes(std::cmp::max(
                *left_length,
                *right_length,
            )));
        }
        _ => (),
    }

    let (left_len, left_signed) = get_int_length(l, l_loc, false, ns, errors)?;

    let (right_len, right_signed) = get_int_length(r, r_loc, false, ns, errors)?;

    Ok(match (left_signed, right_signed) {
        (true, true) => resolver::Type::Int(cmp::max(left_len, right_len)),
        (false, false) => resolver::Type::Uint(cmp::max(left_len, right_len)),
        (true, false) => resolver::Type::Int(cmp::max(left_len, cmp::min(right_len + 8, 256))),
        (false, true) => resolver::Type::Int(cmp::max(cmp::min(left_len + 8, 256), right_len)),
    })
}

/// Try to convert a BigInt into a Expression::NumberLiteral. This checks for sign,
/// width and creates to correct Type.
fn bigint_to_expression(
    loc: &ast::Loc,
    n: &BigInt,
    errors: &mut Vec<Output>,
) -> Result<(Expression, resolver::Type), ()> {
    // Return smallest type
    let bits = n.bits();

    let int_size = if bits < 7 { 8 } else { (bits + 7) & !7 } as u16;

    if n.sign() == Sign::Minus {
        if bits > 255 {
            errors.push(Output::error(*loc, format!("{} is too large", n)));
            Err(())
        } else {
            Ok((
                Expression::NumberLiteral(*loc, int_size, n.clone()),
                resolver::Type::Int(int_size),
            ))
        }
    } else if bits > 256 {
        errors.push(Output::error(*loc, format!("{} is too large", n)));
        Err(())
    } else {
        Ok((
            Expression::NumberLiteral(*loc, int_size, n.clone()),
            resolver::Type::Uint(int_size),
        ))
    }
}

/// Cast from one type to another, which also automatically derefs any Type::Ref() type.
/// if the cast is explicit (e.g. bytes32(bar) then implicit should be set to false.
pub fn cast(
    loc: &ast::Loc,
    expr: Expression,
    from: &resolver::Type,
    to: &resolver::Type,
    implicit: bool,
    ns: &resolver::Namespace,
    errors: &mut Vec<output::Output>,
) -> Result<Expression, ()> {
    if from == to {
        return Ok(expr);
    }

    // First of all, if we have a ref then derefence it
    if let resolver::Type::Ref(r) = from {
        return cast(
            loc,
            Expression::Load(*loc, Box::new(expr)),
            r,
            to,
            implicit,
            ns,
            errors,
        );
    }

    // If it's a storage reference then load the value. The expr is the storage slot
    if let resolver::Type::StorageRef(r) = from {
        if let Expression::StorageBytesSubscript(_, _, _) = expr {
            return cast(loc, expr, r, to, implicit, ns, errors);
        } else {
            return cast(
                loc,
                Expression::StorageLoad(*loc, *r.clone(), Box::new(expr)),
                r,
                to,
                implicit,
                ns,
                errors,
            );
        }
    }

    if from == to {
        return Ok(expr);
    }

    let (from_conv, to_conv) = {
        if implicit {
            (from.clone(), to.clone())
        } else {
            let from_conv = if let resolver::Type::Enum(n) = from {
                ns.enums[*n].ty.clone()
            } else {
                from.clone()
            };

            let to_conv = if let resolver::Type::Enum(n) = to {
                ns.enums[*n].ty.clone()
            } else {
                to.clone()
            };

            (from_conv, to_conv)
        }
    };

    // Special case: when converting literal sign can change if it fits
    match (&expr, &from_conv, &to_conv) {
        (&Expression::NumberLiteral(_, _, ref n), p, &resolver::Type::Uint(to_len))
            if p.is_primitive() =>
        {
            return if n.sign() == Sign::Minus {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "implicit conversion cannot change negative number to {}",
                        to.to_string(ns)
                    ),
                ));

                Err(())
            } else if n.bits() >= to_len as usize {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "implicit conversion would truncate from {} to {}",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));

                Err(())
            } else {
                Ok(Expression::NumberLiteral(*loc, to_len, n.clone()))
            }
        }
        (&Expression::NumberLiteral(_, _, ref n), p, &resolver::Type::Int(to_len))
            if p.is_primitive() =>
        {
            return if n.bits() >= to_len as usize {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "implicit conversion would truncate from {} to {}",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));

                Err(())
            } else {
                Ok(Expression::NumberLiteral(*loc, to_len, n.clone()))
            }
        }
        // Literal strings can be implicitly lengthened
        (&Expression::BytesLiteral(_, ref bs), p, &resolver::Type::Bytes(to_len))
            if p.is_primitive() =>
        {
            return if bs.len() > to_len as usize {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "implicit conversion would truncate from {} to {}",
                        from.to_string(ns),
                        to.to_string(ns)
                    ),
                ));

                Err(())
            } else {
                let mut bs = bs.to_owned();

                // Add zero's at the end as needed
                bs.resize(to_len as usize, 0);

                Ok(Expression::BytesLiteral(*loc, bs))
            };
        }
        (&Expression::BytesLiteral(loc, ref init), _, &resolver::Type::DynamicBytes)
        | (&Expression::BytesLiteral(loc, ref init), _, &resolver::Type::String) => {
            return Ok(Expression::AllocDynamicArray(
                loc,
                to_conv,
                Box::new(Expression::NumberLiteral(loc, 32, BigInt::from(init.len()))),
                Some(init.clone()),
            ));
        }
        _ => (),
    };

    cast_types(
        loc, expr, from_conv, to_conv, from, to, implicit, ns, errors,
    )
}

/// Do casting between types (no literals)
fn cast_types(
    loc: &ast::Loc,
    expr: Expression,
    from_conv: resolver::Type,
    to_conv: resolver::Type,
    from: &resolver::Type,
    to: &resolver::Type,
    implicit: bool,
    ns: &resolver::Namespace,
    errors: &mut Vec<output::Output>,
) -> Result<Expression, ()> {
    let address_bits = ns.address_length as u16 * 8;

    #[allow(clippy::comparison_chain)]
    match (from_conv, to_conv) {
        (resolver::Type::Bytes(1), resolver::Type::Uint(8)) => Ok(expr),
        (resolver::Type::Uint(8), resolver::Type::Bytes(1)) => Ok(expr),
        (resolver::Type::Uint(from_len), resolver::Type::Uint(to_len)) => {
            match from_len.cmp(&to_len) {
                Ordering::Greater => {
                    if implicit {
                        errors.push(Output::type_error(
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
                Ordering::Equal => Ok(expr),
            }
        }
        (resolver::Type::Int(from_len), resolver::Type::Int(to_len)) => match from_len.cmp(&to_len)
        {
            Ordering::Greater => {
                if implicit {
                    errors.push(Output::type_error(
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
            Ordering::Equal => Ok(expr),
        },
        (resolver::Type::Uint(from_len), resolver::Type::Int(to_len)) if to_len > from_len => {
            Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr)))
        }
        (resolver::Type::Int(from_len), resolver::Type::Uint(to_len)) => {
            if implicit {
                errors.push(Output::type_error(
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
                Ok(expr)
            }
        }
        (resolver::Type::Uint(from_len), resolver::Type::Int(to_len)) => {
            if implicit {
                errors.push(Output::type_error(
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
                Ok(expr)
            }
        }
        // Casting int to address
        (resolver::Type::Uint(from_len), resolver::Type::Address(_))
        | (resolver::Type::Int(from_len), resolver::Type::Address(_)) => {
            if implicit {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "implicit conversion from {} to address not allowed",
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if from_len > address_bits {
                Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
            } else if from_len < address_bits {
                Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr)))
            } else {
                Ok(expr)
            }
        }
        // Casting int address to int
        (resolver::Type::Address(_), resolver::Type::Uint(to_len))
        | (resolver::Type::Address(_), resolver::Type::Int(to_len)) => {
            if implicit {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from address not allowed",
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if to_len < address_bits {
                Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
            } else if to_len > address_bits {
                Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr)))
            } else {
                Ok(expr)
            }
        }
        // Lengthing or shorting a fixed bytes array
        (resolver::Type::Bytes(from_len), resolver::Type::Bytes(to_len)) => {
            if implicit {
                errors.push(Output::type_error(
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
                    Box::new(Expression::ZeroExt(*loc, to.clone(), Box::new(expr))),
                    Box::new(Expression::NumberLiteral(
                        *loc,
                        to_len as u16 * 8,
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
                        Box::new(expr),
                        Box::new(Expression::NumberLiteral(
                            *loc,
                            from_len as u16 * 8,
                            BigInt::from_u8(shift).unwrap(),
                        )),
                        false,
                    )),
                ))
            }
        }
        // Explicit conversion from bytesN to int/uint only allowed with expliciy
        // cast and if it is the same size (i.e. no conversion required)
        (resolver::Type::Bytes(from_len), resolver::Type::Uint(to_len))
        | (resolver::Type::Bytes(from_len), resolver::Type::Int(to_len)) => {
            if implicit {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if from_len as u16 * 8 != to_len {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(expr)
            }
        }
        // Explicit conversion to bytesN from int/uint only allowed with expliciy
        // cast and if it is the same size (i.e. no conversion required)
        (resolver::Type::Uint(from_len), resolver::Type::Bytes(to_len))
        | (resolver::Type::Int(from_len), resolver::Type::Bytes(to_len)) => {
            if implicit {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if to_len as u16 * 8 != from_len {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(expr)
            }
        }
        // Explicit conversion from bytesN to address only allowed with expliciy
        // cast and if it is the same size (i.e. no conversion required)
        (resolver::Type::Bytes(from_len), resolver::Type::Address(_)) => {
            if implicit {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if from_len as usize != ns.address_length {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(expr)
            }
        }
        // Implicit conversion to bytesN from int/uint is allowed
        (resolver::Type::Uint(from_len), resolver::Type::Bytes(to_len))
        | (resolver::Type::Int(from_len), resolver::Type::Bytes(to_len)) => Ok(expr),
        // Implicit conversion from bytesN to int/uint is allowed
        (resolver::Type::Bytes(from_len), resolver::Type::Uint(to_len))
        | (resolver::Type::Bytes(from_len), resolver::Type::Int(to_len)) => Ok(expr),
        // Implicit conversion between contract and address is allowed
        (resolver::Type::Contract(_), resolver::Type::Address(false)) => Ok(expr),
        (resolver::Type::Address(_), resolver::Type::Contract(_))
        | (resolver::Type::Contract(_), resolver::Type::Address(true))
        | (resolver::Type::Address(_), resolver::Type::Address(_)) => {
            if implicit {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(expr)
            }
        }
        // Explicit conversion to bytesN from int/uint only allowed with expliciy
        // cast and if it is the same size (i.e. no conversion required)
        (resolver::Type::Address(_), resolver::Type::Bytes(to_len)) => {
            if implicit {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if to_len as usize != ns.address_length {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "conversion to {} from {} not allowed",
                        to.to_string(ns),
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else {
                Ok(expr)
            }
        }
        (resolver::Type::String, resolver::Type::DynamicBytes)
        | (resolver::Type::DynamicBytes, resolver::Type::String)
            if !implicit =>
        {
            Ok(expr)
        }
        // string conversions
        /*
        (resolver::Type::Bytes(_), resolver::Type::String) => Ok(expr),
        (resolver::Type::String, resolver::Type::Bytes(to_len)) => {
            if let Expression::BytesLiteral(_, from_str) = &expr {
                if from_str.len() > to_len as usize {
                    errors.push(Output::type_error(
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
            Ok(expr)
        }
        */
        (resolver::Type::Undef, _) => {
            errors.push(Output::type_error(
                *loc,
                "function or method does not return a value".to_string(),
            ));
            Err(())
        }
        _ => {
            errors.push(Output::type_error(
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

pub fn expression(
    expr: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    match expr {
        ast::Expression::ArrayLiteral(loc, exprs) => {
            resolve_array_literal(loc, exprs, cfg, contract_no, ns, vartab, errors)
        }
        ast::Expression::BoolLiteral(loc, v) => {
            Ok((Expression::BoolLiteral(*loc, *v), resolver::Type::Bool))
        }
        ast::Expression::StringLiteral(v) => {
            // Concatenate the strings
            let mut result = Vec::new();
            let mut loc = ast::Loc(v[0].loc.0, 0);

            for s in v {
                result.extend_from_slice(unescape(&s.string, s.loc.0, errors).as_bytes());
                loc.1 = s.loc.1;
            }

            let length = result.len();

            Ok((
                Expression::BytesLiteral(loc, result),
                resolver::Type::Bytes(length as u8),
            ))
        }
        ast::Expression::HexLiteral(v) => {
            let mut result = Vec::new();
            let mut loc = ast::Loc(0, 0);

            for s in v {
                if (s.hex.len() % 2) != 0 {
                    errors.push(Output::error(
                        s.loc,
                        format!("hex string \"{}\" has odd number of characters", s.hex),
                    ));
                    return Err(());
                } else {
                    result.extend_from_slice(&hex::decode(&s.hex).unwrap());
                    if loc.0 == 0 {
                        loc.0 = s.loc.0;
                    }
                    loc.1 = s.loc.1;
                }
            }

            let length = result.len();

            Ok((
                Expression::BytesLiteral(loc, result),
                resolver::Type::Bytes(length as u8),
            ))
        }
        ast::Expression::NumberLiteral(loc, b) => bigint_to_expression(loc, b, errors),
        ast::Expression::HexNumberLiteral(loc, n) => {
            // ns.address_length is in bytes; double for hex and two for the leading 0x
            let looks_like_address = n.len() == ns.address_length * 2 + 2
                && n.starts_with("0x")
                && !n.chars().any(|c| c == '_');

            if looks_like_address {
                let address = to_hexstr_eip55(n);

                if address == *n {
                    let s: String = address.chars().skip(2).collect();

                    Ok((
                        Expression::NumberLiteral(
                            *loc,
                            ns.address_length as u16 * 8,
                            BigInt::from_str_radix(&s, 16).unwrap(),
                        ),
                        resolver::Type::Address(false),
                    ))
                } else {
                    errors.push(Output::error(
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

                bigint_to_expression(loc, &BigInt::from_str_radix(&s, 16).unwrap(), errors)
            }
        }
        ast::Expression::Variable(id) => {
            if let Some(ref mut tab) = *vartab {
                let v = tab.find(id, contract_no.unwrap(), ns, errors)?;
                match &v.storage {
                    Storage::Contract(n) => Ok((
                        Expression::NumberLiteral(id.loc, 256, n.clone()),
                        resolver::Type::StorageRef(Box::new(v.ty)),
                    )),
                    Storage::Constant(n) => {
                        cfg.add(
                            tab,
                            Instr::Constant {
                                res: v.pos,
                                constant: *n,
                            },
                        );
                        Ok((Expression::Variable(id.loc, v.pos), v.ty))
                    }
                    Storage::Local => Ok((Expression::Variable(id.loc, v.pos), v.ty)),
                }
            } else {
                errors.push(Output::error(
                    id.loc,
                    format!("cannot read variable ‘{}’ in constant expression", id.name),
                ));
                Err(())
            }
        }
        ast::Expression::Add(loc, l, r) => {
            addition(loc, l, r, cfg, contract_no, ns, vartab, errors)
        }
        ast::Expression::Subtract(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

            let ty = coerce_int(
                &left_type,
                &l.loc(),
                &right_type,
                &r.loc(),
                false,
                ns,
                errors,
            )?;

            Ok((
                Expression::Subtract(
                    *loc,
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                ty,
            ))
        }
        ast::Expression::BitwiseOr(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

            let ty = coerce_int(
                &left_type,
                &l.loc(),
                &right_type,
                &r.loc(),
                true,
                ns,
                errors,
            )?;

            Ok((
                Expression::BitwiseOr(
                    *loc,
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                ty,
            ))
        }
        ast::Expression::BitwiseAnd(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

            let ty = coerce_int(
                &left_type,
                &l.loc(),
                &right_type,
                &r.loc(),
                true,
                ns,
                errors,
            )?;

            Ok((
                Expression::BitwiseAnd(
                    *loc,
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                ty,
            ))
        }
        ast::Expression::BitwiseXor(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

            let ty = coerce_int(
                &left_type,
                &l.loc(),
                &right_type,
                &r.loc(),
                true,
                ns,
                errors,
            )?;

            Ok((
                Expression::BitwiseXor(
                    *loc,
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                ty,
            ))
        }
        ast::Expression::ShiftLeft(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

            // left hand side may be bytes/int/uint
            // right hand size may be int/uint
            let _ = get_int_length(&left_type, &l.loc(), true, ns, errors)?;
            let (right_length, _) = get_int_length(&right_type, &r.loc(), false, ns, errors)?;

            Ok((
                Expression::ShiftLeft(
                    *loc,
                    Box::new(left),
                    Box::new(cast_shift_arg(loc, right, right_length, &left_type, ns)),
                ),
                left_type,
            ))
        }
        ast::Expression::ShiftRight(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

            // left hand side may be bytes/int/uint
            // right hand size may be int/uint
            let _ = get_int_length(&left_type, &l.loc(), true, ns, errors)?;
            let (right_length, _) = get_int_length(&right_type, &r.loc(), false, ns, errors)?;

            Ok((
                Expression::ShiftRight(
                    *loc,
                    Box::new(left),
                    Box::new(cast_shift_arg(loc, right, right_length, &left_type, ns)),
                    left_type.signed(),
                ),
                left_type,
            ))
        }
        ast::Expression::Multiply(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

            let ty = coerce_int(
                &left_type,
                &l.loc(),
                &right_type,
                &r.loc(),
                false,
                ns,
                errors,
            )?;

            Ok((
                Expression::Multiply(
                    *loc,
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                ty,
            ))
        }
        ast::Expression::Divide(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

            let ty = coerce_int(
                &left_type,
                &l.loc(),
                &right_type,
                &r.loc(),
                false,
                ns,
                errors,
            )?;

            if ty.signed() {
                Ok((
                    Expression::SDivide(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    ty,
                ))
            } else {
                Ok((
                    Expression::UDivide(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    ty,
                ))
            }
        }
        ast::Expression::Modulo(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

            let ty = coerce_int(
                &left_type,
                &l.loc(),
                &right_type,
                &r.loc(),
                false,
                ns,
                errors,
            )?;

            if ty.signed() {
                Ok((
                    Expression::SModulo(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    ty,
                ))
            } else {
                Ok((
                    Expression::UModulo(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    ty,
                ))
            }
        }
        ast::Expression::Power(loc, b, e) => {
            let (base, base_type) = expression(b, cfg, contract_no, ns, vartab, errors)?;
            let (exp, exp_type) = expression(e, cfg, contract_no, ns, vartab, errors)?;

            // solc-0.5.13 does not allow either base or exp to be signed
            if base_type.signed() || exp_type.signed() {
                errors.push(Output::error(
                    *loc,
                    "exponation (**) is not allowed with signed types".to_string(),
                ));
                return Err(());
            }

            let ty = coerce_int(&base_type, &b.loc(), &exp_type, &e.loc(), false, ns, errors)?;

            Ok((
                Expression::Power(
                    *loc,
                    Box::new(cast(&b.loc(), base, &base_type, &ty, true, ns, errors)?),
                    Box::new(cast(&e.loc(), exp, &exp_type, &ty, true, ns, errors)?),
                ),
                ty,
            ))
        }

        // compare
        ast::Expression::More(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

            let ty = coerce_int(
                &left_type,
                &l.loc(),
                &right_type,
                &r.loc(),
                true,
                ns,
                errors,
            )?;

            if ty.signed() {
                Ok((
                    Expression::SMore(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::Bool,
                ))
            } else {
                Ok((
                    Expression::UMore(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::Bool,
                ))
            }
        }
        ast::Expression::Less(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

            let ty = coerce_int(
                &left_type,
                &l.loc(),
                &right_type,
                &r.loc(),
                true,
                ns,
                errors,
            )?;

            if ty.signed() {
                Ok((
                    Expression::SLess(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::Bool,
                ))
            } else {
                Ok((
                    Expression::ULess(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::Bool,
                ))
            }
        }
        ast::Expression::MoreEqual(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

            let ty = coerce_int(
                &left_type,
                &l.loc(),
                &right_type,
                &r.loc(),
                true,
                ns,
                errors,
            )?;

            if ty.signed() {
                Ok((
                    Expression::SMoreEqual(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::Bool,
                ))
            } else {
                Ok((
                    Expression::UMoreEqual(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::Bool,
                ))
            }
        }
        ast::Expression::LessEqual(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

            let ty = coerce_int(
                &left_type,
                &l.loc(),
                &right_type,
                &r.loc(),
                true,
                ns,
                errors,
            )?;

            if ty.signed() {
                Ok((
                    Expression::SLessEqual(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::Bool,
                ))
            } else {
                Ok((
                    Expression::ULessEqual(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::Bool,
                ))
            }
        }
        ast::Expression::Equal(loc, l, r) => Ok((
            equal(loc, l, r, cfg, contract_no, ns, vartab, errors)?,
            resolver::Type::Bool,
        )),
        ast::Expression::NotEqual(loc, l, r) => Ok((
            Expression::Not(
                *loc,
                Box::new(equal(loc, l, r, cfg, contract_no, ns, vartab, errors)?),
            ),
            resolver::Type::Bool,
        )),
        // unary expressions
        ast::Expression::Not(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, contract_no, ns, vartab, errors)?;

            Ok((
                Expression::Not(
                    *loc,
                    Box::new(cast(
                        &loc,
                        expr,
                        &expr_type,
                        &resolver::Type::Bool,
                        true,
                        ns,
                        errors,
                    )?),
                ),
                resolver::Type::Bool,
            ))
        }
        ast::Expression::Complement(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, contract_no, ns, vartab, errors)?;

            get_int_length(&expr_type, loc, true, ns, errors)?;

            Ok((Expression::Complement(*loc, Box::new(expr)), expr_type))
        }
        ast::Expression::UnaryMinus(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, contract_no, ns, vartab, errors)?;

            if let Expression::NumberLiteral(_, _, n) = expr {
                bigint_to_expression(loc, &-n, errors)
            } else {
                get_int_length(&expr_type, loc, false, ns, errors)?;

                Ok((Expression::UnaryMinus(*loc, Box::new(expr)), expr_type))
            }
        }
        ast::Expression::UnaryPlus(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, contract_no, ns, vartab, errors)?;

            get_int_length(&expr_type, loc, false, ns, errors)?;

            Ok((expr, expr_type))
        }

        ast::Expression::Ternary(loc, c, l, r) => {
            let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;
            let (cond, cond_type) = expression(c, cfg, contract_no, ns, vartab, errors)?;

            let cond = cast(
                &c.loc(),
                cond,
                &cond_type,
                &resolver::Type::Bool,
                true,
                ns,
                errors,
            )?;

            let ty = coerce(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((
                Expression::Ternary(*loc, Box::new(cond), Box::new(left), Box::new(right)),
                ty,
            ))
        }

        // pre/post decrement/increment
        ast::Expression::PostIncrement(loc, var)
        | ast::Expression::PreIncrement(loc, var)
        | ast::Expression::PostDecrement(loc, var)
        | ast::Expression::PreDecrement(loc, var) => {
            let id = match var.as_ref() {
                ast::Expression::Variable(id) => id,
                _ => unreachable!(),
            };

            let vartab = match vartab {
                &mut Some(ref mut tab) => tab,
                None => {
                    errors.push(Output::error(
                        *loc,
                        format!("cannot access variable {} in constant expression", id.name),
                    ));
                    return Err(());
                }
            };

            let v = vartab.find(id, contract_no.unwrap(), ns, errors)?;

            match v.ty {
                resolver::Type::Bytes(_) | resolver::Type::Int(_) | resolver::Type::Uint(_) => (),
                _ => {
                    errors.push(Output::error(
                        var.loc(),
                        format!(
                            "variable ‘{}’ of incorrect type {}",
                            id.name.to_string(),
                            v.ty.to_string(ns)
                        ),
                    ));
                    return Err(());
                }
            };

            let lvalue = match &v.storage {
                Storage::Contract(n) => Expression::StorageLoad(
                    *loc,
                    v.ty.clone(),
                    Box::new(Expression::NumberLiteral(*loc, 256, n.clone())),
                ),
                Storage::Constant(_) => {
                    errors.push(Output::error(
                        *loc,
                        format!("cannot assign to constant ‘{}’", id.name),
                    ));
                    return Err(());
                }
                Storage::Local => Expression::Variable(id.loc, v.pos),
            };

            get_int_length(&v.ty, loc, false, ns, errors)?;

            match expr {
                ast::Expression::PostIncrement(_, _) => {
                    // temporary to hold the value of the variable _before_ incrementing
                    // which will be returned by the expression
                    let temp_pos = vartab.temp(id, &v.ty);
                    cfg.add(
                        vartab,
                        Instr::Set {
                            res: temp_pos,
                            expr: lvalue,
                        },
                    );
                    cfg.add(
                        vartab,
                        Instr::Set {
                            res: v.pos,
                            expr: Expression::Add(
                                *loc,
                                Box::new(Expression::Variable(id.loc, v.pos)),
                                Box::new(Expression::NumberLiteral(
                                    *loc,
                                    v.ty.bits(ns),
                                    One::one(),
                                )),
                            ),
                        },
                    );

                    if let Storage::Contract(n) = &v.storage {
                        cfg.writes_contract_storage = true;
                        cfg.add(
                            vartab,
                            Instr::SetStorage {
                                ty: v.ty.clone(),
                                local: v.pos,
                                storage: Expression::NumberLiteral(*loc, 256, n.clone()),
                            },
                        );
                    }

                    Ok((Expression::Variable(id.loc, temp_pos), v.ty))
                }
                ast::Expression::PostDecrement(_, _) => {
                    // temporary to hold the value of the variable _before_ decrementing
                    // which will be returned by the expression
                    let temp_pos = vartab.temp(id, &v.ty);
                    cfg.add(
                        vartab,
                        Instr::Set {
                            res: temp_pos,
                            expr: lvalue,
                        },
                    );
                    cfg.add(
                        vartab,
                        Instr::Set {
                            res: v.pos,
                            expr: Expression::Subtract(
                                *loc,
                                Box::new(Expression::Variable(id.loc, temp_pos)),
                                Box::new(Expression::NumberLiteral(
                                    *loc,
                                    v.ty.bits(ns),
                                    One::one(),
                                )),
                            ),
                        },
                    );

                    if let Storage::Contract(n) = &v.storage {
                        cfg.writes_contract_storage = true;
                        cfg.add(
                            vartab,
                            Instr::SetStorage {
                                ty: v.ty.clone(),
                                local: v.pos,
                                storage: Expression::NumberLiteral(*loc, 256, n.clone()),
                            },
                        );
                    }

                    Ok((Expression::Variable(id.loc, temp_pos), v.ty))
                }
                ast::Expression::PreIncrement(_, _) => {
                    let temp_pos = vartab.temp(id, &v.ty);
                    cfg.add(
                        vartab,
                        Instr::Set {
                            res: v.pos,
                            expr: Expression::Add(
                                *loc,
                                Box::new(lvalue),
                                Box::new(Expression::NumberLiteral(
                                    *loc,
                                    v.ty.bits(ns),
                                    One::one(),
                                )),
                            ),
                        },
                    );
                    cfg.add(
                        vartab,
                        Instr::Set {
                            res: temp_pos,
                            expr: Expression::Variable(id.loc, v.pos),
                        },
                    );

                    if let Storage::Contract(n) = &v.storage {
                        cfg.writes_contract_storage = true;
                        cfg.add(
                            vartab,
                            Instr::SetStorage {
                                ty: v.ty.clone(),
                                local: v.pos,
                                storage: Expression::NumberLiteral(*loc, 256, n.clone()),
                            },
                        );
                    }

                    Ok((Expression::Variable(id.loc, temp_pos), v.ty))
                }
                ast::Expression::PreDecrement(_, _) => {
                    let temp_pos = vartab.temp(id, &v.ty);
                    cfg.add(
                        vartab,
                        Instr::Set {
                            res: v.pos,
                            expr: Expression::Subtract(
                                *loc,
                                Box::new(lvalue),
                                Box::new(Expression::NumberLiteral(
                                    *loc,
                                    v.ty.bits(ns),
                                    One::one(),
                                )),
                            ),
                        },
                    );
                    cfg.add(
                        vartab,
                        Instr::Set {
                            res: temp_pos,
                            expr: Expression::Variable(id.loc, v.pos),
                        },
                    );

                    if let Storage::Contract(n) = &v.storage {
                        cfg.writes_contract_storage = true;
                        cfg.add(
                            vartab,
                            Instr::SetStorage {
                                ty: v.ty.clone(),
                                local: v.pos,
                                storage: Expression::NumberLiteral(*loc, 256, n.clone()),
                            },
                        );
                    }

                    Ok((Expression::Variable(id.loc, temp_pos), v.ty))
                }
                _ => unreachable!(),
            }
        }

        // assignment
        ast::Expression::Assign(loc, var, e) => {
            assign(loc, var, e, cfg, contract_no, ns, vartab, errors)
        }

        ast::Expression::AssignAdd(loc, var, e)
        | ast::Expression::AssignSubtract(loc, var, e)
        | ast::Expression::AssignMultiply(loc, var, e)
        | ast::Expression::AssignDivide(loc, var, e)
        | ast::Expression::AssignModulo(loc, var, e)
        | ast::Expression::AssignOr(loc, var, e)
        | ast::Expression::AssignAnd(loc, var, e)
        | ast::Expression::AssignXor(loc, var, e)
        | ast::Expression::AssignShiftLeft(loc, var, e)
        | ast::Expression::AssignShiftRight(loc, var, e) => {
            assign_expr(loc, var, expr, e, cfg, contract_no, ns, vartab, errors)
        }
        ast::Expression::NamedFunctionCall(loc, ty, args) => {
            if vartab.is_none() {
                errors.push(Output::error(
                    expr.loc(),
                    "cannot call function in constant expression".to_string(),
                ));
                return Err(());
            }

            let mut blackhole = Vec::new();

            match ns.resolve_type(contract_no, true, ty, &mut blackhole) {
                Ok(resolver::Type::Struct(n)) => {
                    return named_struct_literal(
                        loc,
                        n,
                        args,
                        cfg,
                        contract_no,
                        ns,
                        vartab,
                        errors,
                    );
                }
                Ok(_) => {
                    errors.push(Output::error(
                        *loc,
                        "struct or function expected".to_string(),
                    ));
                    return Err(());
                }
                _ => {}
            }

            let expr =
                named_function_call_expr(loc, ty, args, cfg, contract_no, ns, vartab, errors)?;

            let mut returns =
                emit_function_call(expr.0, expr.1, contract_no.unwrap(), cfg, ns, vartab);

            if returns.len() > 1 {
                errors.push(Output::error(
                    *loc,
                    "in expression context a function cannot return more than one value"
                        .to_string(),
                ));
                return Err(());
            }

            Ok(returns.remove(0))
        }

        ast::Expression::New(loc, call) => match call.as_ref() {
            ast::Expression::FunctionCall(_, ty, args) => {
                let (expr, expr_ty) = new(loc, ty, args, cfg, contract_no, ns, vartab, errors)?;

                Ok(emit_constructor_call(expr, expr_ty, cfg, vartab))
            }
            ast::Expression::NamedFunctionCall(_, ty, args) => {
                let (expr, expr_ty) =
                    constructor_named_args(loc, ty, args, cfg, contract_no, ns, vartab, errors)?;

                Ok(emit_constructor_call(expr, expr_ty, cfg, vartab))
            }
            _ => unreachable!(),
        },
        ast::Expression::Delete(loc, var) => delete(loc, var, cfg, contract_no, ns, vartab, errors),
        ast::Expression::FunctionCall(loc, ty, args) => {
            let mut blackhole = Vec::new();

            match ns.resolve_type(contract_no, true, ty, &mut blackhole) {
                Ok(resolver::Type::Struct(n)) => {
                    return struct_literal(loc, n, args, cfg, contract_no, ns, vartab, errors);
                }
                Ok(to) => {
                    // Cast
                    return if args.is_empty() {
                        errors.push(Output::error(*loc, "missing argument to cast".to_string()));
                        Err(())
                    } else if args.len() > 1 {
                        errors.push(Output::error(
                            *loc,
                            "too many arguments to cast".to_string(),
                        ));
                        Err(())
                    } else {
                        let (expr, expr_type) =
                            expression(&args[0], cfg, contract_no, ns, vartab, errors)?;

                        Ok((cast(loc, expr, &expr_type, &to, false, ns, errors)?, to))
                    };
                }
                Err(_) => {}
            }

            if vartab.is_none() {
                errors.push(Output::error(
                    expr.loc(),
                    "cannot call function in constant expression".to_string(),
                ));
                return Err(());
            }

            let expr = function_call_expr(loc, ty, args, cfg, contract_no, ns, vartab, errors)?;

            let mut returns =
                emit_function_call(expr.0, expr.1, contract_no.unwrap(), cfg, ns, vartab);

            if returns.len() > 1 {
                errors.push(Output::error(
                    *loc,
                    "in expression context a function cannot return more than one value"
                        .to_string(),
                ));
                return Err(());
            }

            Ok(returns.remove(0))
        }
        ast::Expression::ArraySubscript(loc, _, None) => {
            errors.push(Output::error(
                *loc,
                "expected expression before ‘]’ token".to_string(),
            ));

            Err(())
        }
        ast::Expression::ArraySubscript(loc, array, Some(index)) => {
            array_subscript(loc, array, index, cfg, contract_no, ns, vartab, errors)
        }
        ast::Expression::MemberAccess(loc, e, id) => {
            // is of the form "contract_name.enum_name.enum_value"
            if let ast::Expression::MemberAccess(_, e, enum_name) = e.as_ref() {
                if let ast::Expression::Variable(contract_name) = e.as_ref() {
                    if let Some(contract_no) = ns.resolve_contract(contract_name) {
                        if let Some(e) = ns.resolve_enum(Some(contract_no), enum_name) {
                            return match ns.enums[e].values.get(&id.name) {
                                Some((_, val)) => Ok((
                                    Expression::NumberLiteral(
                                        *loc,
                                        ns.enums[e].ty.bits(ns),
                                        BigInt::from_usize(*val).unwrap(),
                                    ),
                                    resolver::Type::Enum(e),
                                )),
                                None => {
                                    errors.push(Output::error(
                                        id.loc,
                                        format!(
                                            "enum {} does not have value {}",
                                            ns.enums[e].print_to_string(),
                                            id.name
                                        ),
                                    ));
                                    Err(())
                                }
                            };
                        }
                    }
                }
            }

            // is of the form "enum_name.enum_value"
            if let ast::Expression::Variable(namespace) = e.as_ref() {
                if let Some(e) = ns.resolve_enum(contract_no, namespace) {
                    return match ns.enums[e].values.get(&id.name) {
                        Some((_, val)) => Ok((
                            Expression::NumberLiteral(
                                *loc,
                                ns.enums[e].ty.bits(ns),
                                BigInt::from_usize(*val).unwrap(),
                            ),
                            resolver::Type::Enum(e),
                        )),
                        None => {
                            errors.push(Output::error(
                                id.loc,
                                format!(
                                    "enum {} does not have value {}",
                                    ns.enums[e].print_to_string(),
                                    id.name
                                ),
                            ));
                            Err(())
                        }
                    };
                }
            }

            // is of the form "type(x).field", like type(c).min
            if let ast::Expression::FunctionCall(_, name, args) = e.as_ref() {
                if let ast::Expression::Variable(func_name) = name.as_ref() {
                    if func_name.name == "type" {
                        return type_name_expr(loc, args, id, contract_no, ns, errors);
                    }
                }
            }

            let (expr, expr_ty) = expression(e, cfg, contract_no, ns, vartab, errors)?;

            // Dereference if need to. This could be struct-in-struct for
            // example.
            let (expr, expr_ty) = if let resolver::Type::Ref(ty) = expr_ty {
                (Expression::Load(*loc, Box::new(expr)), *ty)
            } else {
                (expr, expr_ty)
            };

            match expr_ty {
                resolver::Type::Bytes(n) => {
                    if id.name == "length" {
                        return Ok((
                            Expression::NumberLiteral(*loc, 8, BigInt::from_u8(n).unwrap()),
                            resolver::Type::Uint(8),
                        ));
                    }
                }
                resolver::Type::Array(_, dim) => {
                    if id.name == "length" {
                        return match dim.last().unwrap() {
                            None => Ok((
                                Expression::DynamicArrayLength(*loc, Box::new(expr)),
                                resolver::Type::Uint(32),
                            )),
                            Some(d) => bigint_to_expression(loc, d, errors),
                        };
                    }
                }
                resolver::Type::String | resolver::Type::DynamicBytes => {
                    if id.name == "length" {
                        return Ok((
                            Expression::DynamicArrayLength(*loc, Box::new(expr)),
                            resolver::Type::Uint(32),
                        ));
                    }
                }
                resolver::Type::StorageRef(r) => match *r {
                    resolver::Type::Struct(n) => {
                        let mut slot = BigInt::zero();

                        for field in &ns.structs[n].fields {
                            if id.name == field.name {
                                return Ok((
                                    Expression::Add(
                                        *loc,
                                        Box::new(expr),
                                        Box::new(Expression::NumberLiteral(*loc, 256, slot)),
                                    ),
                                    resolver::Type::StorageRef(Box::new(field.ty.clone())),
                                ));
                            }

                            slot += field.ty.storage_slots(ns);
                        }

                        errors.push(Output::error(
                            id.loc,
                            format!(
                                "struct ‘{}’ does not have a field called ‘{}’",
                                ns.structs[n].name, id.name
                            ),
                        ));
                        return Err(());
                    }
                    resolver::Type::Bytes(n) => {
                        if id.name == "length" {
                            return Ok((
                                Expression::NumberLiteral(*loc, 8, BigInt::from_u8(n).unwrap()),
                                resolver::Type::Uint(8),
                            ));
                        }
                    }
                    resolver::Type::Array(_, dim) => {
                        if id.name == "length" {
                            return match dim.last().unwrap() {
                                None => Ok((
                                    expr,
                                    resolver::Type::StorageRef(Box::new(resolver::Type::Uint(256))),
                                )),
                                Some(d) => bigint_to_expression(loc, d, errors),
                            };
                        }
                    }
                    resolver::Type::DynamicBytes => {
                        if id.name == "length" {
                            return Ok((
                                Expression::StorageBytesLength(*loc, Box::new(expr)),
                                resolver::Type::Uint(32),
                            ));
                        }
                    }
                    _ => {}
                },
                resolver::Type::Struct(n) => {
                    if let Some((i, f)) = ns.structs[n]
                        .fields
                        .iter()
                        .enumerate()
                        .find(|f| id.name == f.1.name)
                    {
                        return Ok((
                            Expression::StructMember(*loc, Box::new(expr), i),
                            resolver::Type::Ref(Box::new(f.ty.clone())),
                        ));
                    } else {
                        errors.push(Output::error(
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
                _ => (),
            }

            errors.push(Output::error(*loc, format!("‘{}’ not found", id.name)));

            Err(())
        }
        ast::Expression::Or(loc, left, right) => {
            let boolty = resolver::Type::Bool;
            let (l, l_type) = expression(left, cfg, contract_no, ns, vartab, errors)?;
            let l = cast(&loc, l, &l_type, &boolty, true, ns, errors)?;

            let mut tab = match vartab {
                &mut Some(ref mut tab) => tab,
                None => {
                    // In constant context, no side effects so short-circut not necessary
                    let (r, r_type) = expression(right, cfg, contract_no, ns, vartab, errors)?;

                    return Ok((
                        Expression::Or(
                            *loc,
                            Box::new(l),
                            Box::new(cast(
                                &loc,
                                r,
                                &r_type,
                                &resolver::Type::Bool,
                                true,
                                ns,
                                errors,
                            )?),
                        ),
                        resolver::Type::Bool,
                    ));
                }
            };

            let pos = tab.temp(
                &ast::Identifier {
                    name: "or".to_owned(),
                    loc: *loc,
                },
                &resolver::Type::Bool,
            );

            let right_side = cfg.new_basic_block("or_right_side".to_string());
            let end_or = cfg.new_basic_block("or_end".to_string());

            cfg.add(
                tab,
                Instr::Set {
                    res: pos,
                    expr: Expression::BoolLiteral(*loc, true),
                },
            );
            cfg.add(
                tab,
                Instr::BranchCond {
                    cond: l,
                    true_: end_or,
                    false_: right_side,
                },
            );
            cfg.set_basic_block(right_side);

            let (r, r_type) = expression(right, cfg, contract_no, ns, &mut Some(&mut tab), errors)?;
            let r = cast(&loc, r, &r_type, &resolver::Type::Bool, true, ns, errors)?;

            cfg.add(tab, Instr::Set { res: pos, expr: r });

            let mut phis = HashSet::new();
            phis.insert(pos);

            cfg.set_phis(end_or, phis);

            cfg.add(tab, Instr::Branch { bb: end_or });

            cfg.set_basic_block(end_or);

            Ok((Expression::Variable(*loc, pos), boolty))
        }
        ast::Expression::And(loc, left, right) => {
            let boolty = resolver::Type::Bool;
            let (l, l_type) = expression(left, cfg, contract_no, ns, vartab, errors)?;
            let l = cast(&loc, l, &l_type, &boolty, true, ns, errors)?;

            let mut tab = match vartab {
                &mut Some(ref mut tab) => tab,
                None => {
                    // In constant context, no side effects so short-circut not necessary
                    let (r, r_type) = expression(right, cfg, contract_no, ns, vartab, errors)?;

                    return Ok((
                        Expression::And(
                            *loc,
                            Box::new(l),
                            Box::new(cast(
                                &loc,
                                r,
                                &r_type,
                                &resolver::Type::Bool,
                                true,
                                ns,
                                errors,
                            )?),
                        ),
                        resolver::Type::Bool,
                    ));
                }
            };

            let pos = tab.temp(
                &ast::Identifier {
                    name: "and".to_owned(),
                    loc: *loc,
                },
                &resolver::Type::Bool,
            );

            let right_side = cfg.new_basic_block("and_right_side".to_string());
            let end_and = cfg.new_basic_block("and_end".to_string());

            cfg.add(
                tab,
                Instr::Set {
                    res: pos,
                    expr: Expression::BoolLiteral(*loc, false),
                },
            );
            cfg.add(
                tab,
                Instr::BranchCond {
                    cond: l,
                    true_: right_side,
                    false_: end_and,
                },
            );
            cfg.set_basic_block(right_side);

            let (r, r_type) = expression(right, cfg, contract_no, ns, &mut Some(&mut tab), errors)?;
            let r = cast(&loc, r, &r_type, &resolver::Type::Bool, true, ns, errors)?;

            cfg.add(tab, Instr::Set { res: pos, expr: r });

            let mut phis = HashSet::new();
            phis.insert(pos);

            cfg.set_phis(end_and, phis);

            cfg.add(tab, Instr::Branch { bb: end_and });

            cfg.set_basic_block(end_and);

            Ok((Expression::Variable(*loc, pos), boolty))
        }
        ast::Expression::Type(loc, _) => {
            errors.push(Output::error(*loc, "type not expected".to_owned()));
            Err(())
        }
        ast::Expression::List(loc, _) => {
            errors.push(Output::error(
                *loc,
                "lists only permitted in destructure statements".to_owned(),
            ));
            Err(())
        }
    }
}

/// Resolve an new contract expression with positional arguments
fn constructor(
    loc: &ast::Loc,
    no: usize,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    // The current contract cannot be constructed with new. In order to create
    // the contract, we need the code hash of the contract. Part of that code
    // will be code we're emitted here. So we end up with a crypto puzzle.
    let contract_no = match contract_no {
        Some(n) if n == no => {
            errors.push(Output::error(
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
            errors.push(Output::error(
                *loc,
                "new contract not allowed in this context".to_string(),
            ));
            return Err(());
        }
    };

    // check for circular references
    if circular_reference(no, contract_no, ns) {
        errors.push(Output::error(
            *loc,
            format!(
                "circular reference creating contract ‘{}’",
                ns.contracts[no].name
            ),
        ));
        return Err(());
    }

    let mut creates = ns.contracts[contract_no].creates.borrow_mut();

    if !creates.contains(&no) {
        creates.push(no);
    }

    let mut resolved_args = Vec::new();
    let mut resolved_types = Vec::new();

    for arg in args {
        let (expr, expr_type) = expression(arg, cfg, Some(contract_no), ns, vartab, errors)?;

        resolved_args.push(Box::new(expr));
        resolved_types.push(expr_type);
    }

    let mut temp_errors = Vec::new();

    // constructor call
    for (constructor_no, func) in ns.contracts[no]
        .functions
        .iter()
        .filter(|f| f.is_constructor())
        .enumerate()
    {
        if func.params.len() != args.len() {
            temp_errors.push(Output::error(
                *loc,
                format!(
                    "constructor expects {} arguments, {} provided",
                    func.params.len(),
                    args.len()
                ),
            ));
            continue;
        }

        let mut matches = true;
        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for (i, param) in func.params.iter().enumerate() {
            let arg = &resolved_args[i];

            match cast(
                &args[i].loc(),
                *arg.clone(),
                &resolved_types[i],
                &param.ty,
                true,
                ns,
                &mut temp_errors,
            ) {
                Ok(expr) => cast_args.push(expr),
                Err(()) => {
                    matches = false;
                    break;
                }
            }
        }

        if matches {
            return Ok((
                Expression::Constructor(*loc, no, constructor_no, cast_args),
                resolver::Type::Contract(no),
            ));
        }
    }

    match ns.contracts[no]
        .functions
        .iter()
        .filter(|f| f.is_constructor())
        .count()
    {
        0 => Ok((
            Expression::Constructor(*loc, no, 0, Vec::new()),
            resolver::Type::Contract(no),
        )),
        1 => {
            errors.append(&mut temp_errors);

            Err(())
        }
        _ => {
            errors.push(Output::error(
                *loc,
                "cannot find overloaded constructor which matches signature".to_string(),
            ));

            Err(())
        }
    }
}

/// check if from creates to, recursively
fn circular_reference(from: usize, to: usize, ns: &resolver::Namespace) -> bool {
    let creates = ns.contracts[from].creates.borrow();

    if creates.contains(&to) {
        return true;
    }

    creates.iter().any(|n| circular_reference(*n, to, &ns))
}

/// Resolve an new contract expression with named arguments
pub fn constructor_named_args(
    loc: &ast::Loc,
    ty: &ast::Expression,
    args: &[ast::NamedArgument],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let no = match ns.resolve_type(contract_no, false, ty, errors)? {
        resolver::Type::Contract(n) => n,
        _ => {
            errors.push(Output::error(*loc, "contract expected".to_string()));
            return Err(());
        }
    };

    // The current contract cannot be constructed with new. In order to create
    // the contract, we need the code hash of the contract. Part of that code
    // will be code we're emitted here. So we end up with a crypto puzzle.
    let contract_no = match contract_no {
        Some(n) if n == no => {
            errors.push(Output::error(
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
            errors.push(Output::error(
                *loc,
                "new contract not allowed in this context".to_string(),
            ));
            return Err(());
        }
    };

    // check for circular references
    if circular_reference(no, contract_no, ns) {
        errors.push(Output::error(
            *loc,
            format!(
                "circular reference creating contract ‘{}’",
                ns.contracts[no].name
            ),
        ));
        return Err(());
    }

    let mut creates = ns.contracts[contract_no].creates.borrow_mut();

    if !creates.contains(&no) {
        creates.push(no);
    }

    let mut arguments = HashMap::new();

    for arg in args {
        arguments.insert(
            arg.name.name.to_string(),
            expression(&arg.expr, cfg, Some(contract_no), ns, vartab, errors)?,
        );
    }

    let mut temp_errors = Vec::new();

    // constructor call
    for (constructor_no, func) in ns.contracts[no]
        .functions
        .iter()
        .filter(|f| f.is_constructor())
        .enumerate()
    {
        if func.params.len() != args.len() {
            temp_errors.push(Output::error(
                *loc,
                format!(
                    "constructor expects {} arguments, {} provided",
                    func.params.len(),
                    args.len()
                ),
            ));
            continue;
        }

        let mut matches = true;
        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for param in func.params.iter() {
            let arg = match arguments.get(&param.name) {
                Some(a) => a,
                None => {
                    matches = false;
                    temp_errors.push(Output::error(
                        *loc,
                        format!("missing argument ‘{}’ to constructor", param.name),
                    ));
                    break;
                }
            };

            match cast(
                &ast::Loc(0, 0),
                arg.0.clone(),
                &arg.1,
                &param.ty,
                true,
                ns,
                &mut temp_errors,
            ) {
                Ok(expr) => cast_args.push(expr),
                Err(()) => {
                    matches = false;
                    break;
                }
            }
        }

        if matches {
            return Ok((
                Expression::Constructor(*loc, no, constructor_no, cast_args),
                resolver::Type::Contract(no),
            ));
        }
    }

    match ns.contracts[no]
        .functions
        .iter()
        .filter(|f| f.is_constructor())
        .count()
    {
        0 => Ok((
            Expression::Constructor(*loc, no, 0, Vec::new()),
            resolver::Type::Contract(no),
        )),
        1 => {
            errors.append(&mut temp_errors);

            Err(())
        }
        _ => {
            errors.push(Output::error(
                *loc,
                "cannot find overloaded constructor which matches signature".to_string(),
            ));

            Err(())
        }
    }
}

/// Resolve type(x).foo
pub fn type_name_expr(
    loc: &ast::Loc,
    args: &[ast::Expression],
    field: &ast::Identifier,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    if args.is_empty() {
        errors.push(Output::error(
            *loc,
            "missing argument to type()".to_string(),
        ));
        return Err(());
    }

    if args.len() > 1 {
        errors.push(Output::error(
            *loc,
            format!("got {} arguments to type(), only one expected", args.len(),),
        ));
        return Err(());
    }

    let ty = ns.resolve_type(contract_no, false, &args[0], errors)?;

    match (&ty, field.name.as_str()) {
        (resolver::Type::Uint(_), "min") => bigint_to_expression(loc, &BigInt::zero(), errors),
        (resolver::Type::Uint(bits), "max") => {
            let max = BigInt::one().shl(*bits as usize).sub(1);
            bigint_to_expression(loc, &max, errors)
        }
        (resolver::Type::Int(bits), "min") => {
            let min = BigInt::zero().sub(BigInt::one().shl(*bits as usize - 1));
            bigint_to_expression(loc, &min, errors)
        }
        (resolver::Type::Int(bits), "max") => {
            let max = BigInt::one().shl(*bits as usize - 1).sub(1);
            bigint_to_expression(loc, &max, errors)
        }
        (resolver::Type::Contract(n), "name") => Ok((
            Expression::BytesLiteral(*loc, ns.contracts[*n].name.as_bytes().to_vec()),
            resolver::Type::String,
        )),
        (resolver::Type::Contract(no), "creationCode")
        | (resolver::Type::Contract(no), "runtimeCode") => {
            let contract_no = match contract_no {
                Some(contract_no) => contract_no,
                None => {
                    errors.push(Output::error(
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
                errors.push(Output::error(
                    *loc,
                    format!(
                        "containing our own contract code for ‘{}’ would generate infinite size contract",
                        ns.contracts[*no].name
                    ),
                ));
                return Err(());
            }

            if circular_reference(*no, contract_no, ns) {
                errors.push(Output::error(
                    *loc,
                    format!(
                        "circular reference creating contract code for ‘{}’",
                        ns.contracts[*no].name
                    ),
                ));
                return Err(());
            }

            let mut creates = ns.contracts[contract_no].creates.borrow_mut();

            if !creates.contains(no) {
                creates.push(*no);
            }

            Ok((
                Expression::CodeLiteral(*loc, *no, field.name == "runtimeCode"),
                resolver::Type::DynamicBytes,
            ))
        }
        _ => {
            errors.push(Output::error(
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
    loc: &ast::Loc,
    ty: &ast::Expression,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let ty = ns.resolve_type(contract_no, false, ty, errors)?;

    match &ty {
        resolver::Type::Array(ty, dim) => {
            if dim.last().unwrap().is_some() {
                errors.push(Output::error(
                    *loc,
                    format!(
                        "new cannot allocate fixed array type ‘{}’",
                        ty.to_string(ns)
                    ),
                ));
                return Err(());
            }

            if let resolver::Type::Contract(_) = ty.as_ref() {
                errors.push(Output::error(
                    *loc,
                    format!("new cannot construct array of ‘{}’", ty.to_string(ns)),
                ));
                return Err(());
            }
        }
        resolver::Type::String | resolver::Type::DynamicBytes => {}
        resolver::Type::Contract(n) => {
            return constructor(loc, *n, args, cfg, contract_no, ns, vartab, errors);
        }
        _ => {
            errors.push(Output::error(
                *loc,
                format!("new cannot allocate type ‘{}’", ty.to_string(ns)),
            ));
            return Err(());
        }
    };

    if args.len() != 1 {
        errors.push(Output::error(
            *loc,
            "new dynamic array should have a single length argument".to_string(),
        ));
        return Err(());
    }
    let size_loc = args[0].loc();

    let (size_expr, size_ty) = expression(&args[0], cfg, contract_no, ns, vartab, errors)?;

    let size_width = match size_ty {
        resolver::Type::Uint(n) => n,
        _ => {
            errors.push(Output::error(
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
        Ordering::Greater => {
            Expression::Trunc(size_loc, resolver::Type::Uint(32), Box::new(size_expr))
        }
        Ordering::Less => {
            Expression::ZeroExt(size_loc, resolver::Type::Uint(32), Box::new(size_expr))
        }
        Ordering::Equal => size_expr,
    };

    Ok((
        Expression::AllocDynamicArray(*loc, ty.clone(), Box::new(size), None),
        ty,
    ))
}

/// Test for equality; first check string equality, then integer equality
fn equal(
    loc: &ast::Loc,
    l: &ast::Expression,
    r: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<Expression, ()> {
    let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
    let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

    // Comparing stringliteral against stringliteral
    if let (Expression::BytesLiteral(_, l), Expression::BytesLiteral(_, r)) = (&left, &right) {
        return Ok(Expression::BoolLiteral(*loc, l == r));
    }

    // compare string against literal
    match (&left, &right_type.deref()) {
        (Expression::BytesLiteral(_, l), resolver::Type::String)
        | (Expression::BytesLiteral(_, l), resolver::Type::DynamicBytes) => {
            return Ok(Expression::StringCompare(
                *loc,
                StringLocation::RunTime(Box::new(cast(
                    &r.loc(),
                    right,
                    &right_type,
                    &right_type.deref(),
                    true,
                    ns,
                    errors,
                )?)),
                StringLocation::CompileTime(l.clone()),
            ));
        }
        _ => {}
    }

    match (&right, &left_type.deref()) {
        (Expression::BytesLiteral(_, literal), resolver::Type::String)
        | (Expression::BytesLiteral(_, literal), resolver::Type::DynamicBytes) => {
            return Ok(Expression::StringCompare(
                *loc,
                StringLocation::RunTime(Box::new(cast(
                    &l.loc(),
                    left,
                    &left_type,
                    &left_type.deref(),
                    true,
                    ns,
                    errors,
                )?)),
                StringLocation::CompileTime(literal.clone()),
            ));
        }
        _ => {}
    }

    // compare string
    match (&left_type.deref(), &right_type.deref()) {
        (resolver::Type::String, resolver::Type::String)
        | (resolver::Type::DynamicBytes, resolver::Type::DynamicBytes) => {
            return Ok(Expression::StringCompare(
                *loc,
                StringLocation::RunTime(Box::new(cast(
                    &l.loc(),
                    left,
                    &left_type,
                    &left_type.deref(),
                    true,
                    ns,
                    errors,
                )?)),
                StringLocation::RunTime(Box::new(cast(
                    &r.loc(),
                    right,
                    &right_type,
                    &right_type.deref(),
                    true,
                    ns,
                    errors,
                )?)),
            ));
        }
        _ => {}
    }

    let ty = coerce(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

    Ok(Expression::Equal(
        *loc,
        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
    ))
}

/// Try string concatenation
fn addition(
    loc: &ast::Loc,
    l: &ast::Expression,
    r: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let (left, left_type) = expression(l, cfg, contract_no, ns, vartab, errors)?;
    let (right, right_type) = expression(r, cfg, contract_no, ns, vartab, errors)?;

    // Concatenate stringliteral with stringliteral
    if let (Expression::BytesLiteral(_, l), Expression::BytesLiteral(_, r)) = (&left, &right) {
        let mut c = Vec::with_capacity(l.len() + r.len());
        c.extend_from_slice(l);
        c.extend_from_slice(r);
        let length = c.len();
        return Ok((
            Expression::BytesLiteral(*loc, c),
            resolver::Type::Bytes(length as u8),
        ));
    }

    // compare string against literal
    match (&left, &right_type) {
        (Expression::BytesLiteral(_, l), resolver::Type::String)
        | (Expression::BytesLiteral(_, l), resolver::Type::DynamicBytes) => {
            return Ok((
                Expression::StringConcat(
                    *loc,
                    StringLocation::CompileTime(l.clone()),
                    StringLocation::RunTime(Box::new(right)),
                ),
                right_type,
            ));
        }
        _ => {}
    }

    match (&right, &left_type) {
        (Expression::BytesLiteral(_, l), resolver::Type::String)
        | (Expression::BytesLiteral(_, l), resolver::Type::DynamicBytes) => {
            return Ok((
                Expression::StringConcat(
                    *loc,
                    StringLocation::RunTime(Box::new(left)),
                    StringLocation::CompileTime(l.clone()),
                ),
                left_type,
            ));
        }
        _ => {}
    }

    // compare string
    match (&left_type, &right_type) {
        (resolver::Type::String, resolver::Type::String)
        | (resolver::Type::DynamicBytes, resolver::Type::DynamicBytes) => {
            return Ok((
                Expression::StringConcat(
                    *loc,
                    StringLocation::RunTime(Box::new(left)),
                    StringLocation::RunTime(Box::new(right)),
                ),
                right_type,
            ));
        }
        _ => {}
    }

    let ty = coerce_int(
        &left_type,
        &l.loc(),
        &right_type,
        &r.loc(),
        false,
        ns,
        errors,
    )?;

    Ok((
        Expression::Add(
            *loc,
            Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
            Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
        ),
        ty,
    ))
}

/// Resolve an assignment
fn assign(
    loc: &ast::Loc,
    var: &ast::Expression,
    e: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    // is it a destructuring assignment
    if let ast::Expression::List(_, var) = var {
        destructuring(loc, var, e, cfg, contract_no, ns, vartab, errors)
    } else {
        let (expr, expr_type) = expression(e, cfg, contract_no, ns, vartab, errors)?;

        assign_single(
            loc,
            var,
            expr,
            expr_type,
            cfg,
            contract_no,
            ns,
            vartab,
            errors,
        )
    }
}

/// Resolve an assignment
fn assign_single(
    loc: &ast::Loc,
    var: &ast::Expression,
    expr: Expression,
    expr_type: resolver::Type,
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    match var {
        ast::Expression::Variable(id) => {
            let vartab = match vartab {
                &mut Some(ref mut tab) => tab,
                None => {
                    errors.push(Output::error(
                        *loc,
                        format!(
                            "cannot access variable ‘{}’ in constant expression",
                            id.name
                        ),
                    ));
                    return Err(());
                }
            };
            let var = vartab.find(id, contract_no.unwrap(), ns, errors)?;

            cfg.add(
                vartab,
                Instr::Set {
                    res: var.pos,
                    expr: cast(&id.loc, expr, &expr_type, &var.ty, true, ns, errors)?,
                },
            );

            match &var.storage {
                Storage::Contract(n) => {
                    cfg.writes_contract_storage = true;
                    cfg.add(
                        vartab,
                        Instr::SetStorage {
                            ty: var.ty.clone(),
                            local: var.pos,
                            storage: Expression::NumberLiteral(*loc, 256, n.clone()),
                        },
                    );
                }
                Storage::Constant(_) => {
                    errors.push(Output::error(
                        *loc,
                        format!("cannot assign to constant ‘{}’", id.name),
                    ));
                    return Err(());
                }
                Storage::Local => {
                    // nothing to do
                }
            }

            Ok((Expression::Variable(id.loc, var.pos), var.ty))
        }
        _ => {
            // for example: a[0] = 102
            let (var_expr, var_ty) = expression(var, cfg, contract_no, ns, vartab, errors)?;

            let vartab = match vartab {
                &mut Some(ref mut tab) => tab,
                None => {
                    errors.push(Output::error(
                        *loc,
                        "cannot assign in constant expression".to_string(),
                    ));
                    return Err(());
                }
            };

            match &var_ty {
                resolver::Type::Ref(r_ty) => {
                    let pos = vartab.temp_anonymous(&var_ty);

                    // reference to memory (e.g. array)
                    cfg.add(
                        vartab,
                        Instr::Set {
                            res: pos,
                            expr: cast(&var.loc(), expr, &expr_type, &r_ty, true, ns, errors)?,
                        },
                    );

                    // set the element in memory
                    cfg.add(
                        vartab,
                        Instr::Store {
                            dest: var_expr,
                            pos,
                        },
                    );

                    Ok((Expression::Variable(*loc, pos), r_ty.as_ref().clone()))
                }
                resolver::Type::StorageRef(r_ty) => {
                    let pos = vartab.temp_anonymous(&r_ty);

                    cfg.add(
                        vartab,
                        Instr::Set {
                            res: pos,
                            expr: cast(&var.loc(), expr, &expr_type, &r_ty, true, ns, errors)?,
                        },
                    );

                    if let Expression::StorageBytesSubscript(_, array, index) = var_expr {
                        // Set a byte in a byte array
                        cfg.add(
                            vartab,
                            Instr::SetStorageBytes {
                                local: pos,
                                storage: array,
                                offset: index,
                            },
                        );
                    } else {
                        // The value of the var_expr should be storage offset
                        cfg.add(
                            vartab,
                            Instr::SetStorage {
                                ty: *r_ty.clone(),
                                local: pos,
                                storage: var_expr,
                            },
                        );
                    }
                    cfg.writes_contract_storage = true;

                    Ok((Expression::Variable(*loc, pos), r_ty.as_ref().clone()))
                }
                _ => {
                    errors.push(Output::error(
                        var.loc(),
                        "expression is not assignable".to_string(),
                    ));
                    Err(())
                }
            }
        }
    }
}

/// Resolve an destructuring assignment
fn destructuring(
    loc: &ast::Loc,
    var: &[(ast::Loc, Option<ast::Parameter>)],
    e: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let vartab = match vartab {
        &mut Some(ref mut tab) => tab,
        None => {
            errors.push(Output::error(
                *loc,
                "assignment not allowed in constant context".to_string(),
            ));
            return Err(());
        }
    };

    let mut args = match e {
        ast::Expression::FunctionCall(loc, ty, args) => {
            let expr = function_call_expr(
                loc,
                ty,
                args,
                cfg,
                contract_no,
                ns,
                &mut Some(vartab),
                errors,
            )?;

            emit_function_call(
                expr.0,
                expr.1,
                contract_no.unwrap(),
                cfg,
                ns,
                &mut Some(vartab),
            )
        }
        ast::Expression::NamedFunctionCall(loc, ty, args) => {
            let expr = named_function_call_expr(
                loc,
                ty,
                args,
                cfg,
                contract_no,
                ns,
                &mut Some(vartab),
                errors,
            )?;

            emit_function_call(
                expr.0,
                expr.1,
                contract_no.unwrap(),
                cfg,
                ns,
                &mut Some(vartab),
            )
        }
        _ => {
            let mut list = Vec::new();

            for e in parameter_list_to_expr_list(e, errors)? {
                let (expr, ty) = expression(e, cfg, contract_no, ns, &mut Some(vartab), errors)?;

                // we need to copy the arguments into temps in case there is a swap involved
                // we're assuming that llvm will optimize these away if possible
                let pos = vartab.temp_anonymous(&ty);
                cfg.add(vartab, Instr::Set { res: pos, expr });
                list.push((Expression::Variable(e.loc(), pos), ty));
            }

            list
        }
    };

    if args.len() != var.len() {
        errors.push(Output::error(
            *loc,
            format!(
                "destructuring assignment has {} values on the left and {} on the right",
                var.len(),
                args.len()
            ),
        ));
        return Err(());
    }

    for e in var {
        let (arg, arg_ty) = args.remove(0);

        match &e.1 {
            None => {
                // nothing to do
            }
            Some(ast::Parameter {
                ty,
                storage,
                name: None,
            }) => {
                // so this is a simple assignment, e.g. "(foo, bar) = (1, 2);"
                // both foo and bar should be declared
                assign_single(
                    &e.0,
                    &ty,
                    arg,
                    arg_ty,
                    cfg,
                    contract_no,
                    ns,
                    &mut Some(vartab),
                    errors,
                )?;

                if let Some(storage) = storage {
                    errors.push(Output::error(
                        *storage.loc(),
                        format!("storage modifier ‘{}’ not permitted on assignment", storage),
                    ));
                    return Err(());
                }
            }
            Some(ast::Parameter {
                ty,
                storage,
                name: Some(name),
            }) => {
                let var_ty = resolve_var_decl_ty(&ty, &storage, contract_no, ns, errors)?;

                let expr = cast(&e.0, arg, &arg_ty, &var_ty, true, ns, errors)?;

                if let Some(pos) = vartab.add(&name, var_ty, errors) {
                    ns.check_shadowing(contract_no.unwrap(), &name, errors);

                    cfg.add(vartab, Instr::Set { res: pos, expr });
                }
            }
        }
    }

    Ok((Expression::Poison, resolver::Type::Undef))
}

/// Resolve an assignment with an operator
fn assign_expr(
    loc: &ast::Loc,
    var: &ast::Expression,
    expr: &ast::Expression,
    e: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let (set, set_type) = expression(e, cfg, contract_no, ns, vartab, errors)?;

    let op = |assign: Expression,
              ty: &resolver::Type,
              errors: &mut Vec<output::Output>|
     -> Result<Expression, ()> {
        let set = match expr {
            ast::Expression::AssignShiftLeft(_, _, _)
            | ast::Expression::AssignShiftRight(_, _, _) => {
                let left_length = get_int_length(&ty, &loc, true, ns, errors)?;
                let right_length = get_int_length(&set_type, &e.loc(), false, ns, errors)?;

                // TODO: does shifting by negative value need compiletime/runtime check?
                if left_length == right_length {
                    set
                } else if right_length < left_length && set_type.signed() {
                    Expression::SignExt(*loc, ty.clone(), Box::new(set))
                } else if right_length < left_length && !set_type.signed() {
                    Expression::ZeroExt(*loc, ty.clone(), Box::new(set))
                } else {
                    Expression::Trunc(*loc, ty.clone(), Box::new(set))
                }
            }
            _ => cast(&var.loc(), set, &set_type, &ty, true, ns, errors)?,
        };

        Ok(match expr {
            ast::Expression::AssignAdd(_, _, _) => {
                Expression::Add(*loc, Box::new(assign), Box::new(set))
            }
            ast::Expression::AssignSubtract(_, _, _) => {
                Expression::Subtract(*loc, Box::new(assign), Box::new(set))
            }
            ast::Expression::AssignMultiply(_, _, _) => {
                Expression::Multiply(*loc, Box::new(assign), Box::new(set))
            }
            ast::Expression::AssignOr(_, _, _) => {
                Expression::BitwiseOr(*loc, Box::new(assign), Box::new(set))
            }
            ast::Expression::AssignAnd(_, _, _) => {
                Expression::BitwiseAnd(*loc, Box::new(assign), Box::new(set))
            }
            ast::Expression::AssignXor(_, _, _) => {
                Expression::BitwiseXor(*loc, Box::new(assign), Box::new(set))
            }
            ast::Expression::AssignShiftLeft(_, _, _) => {
                Expression::ShiftLeft(*loc, Box::new(assign), Box::new(set))
            }
            ast::Expression::AssignShiftRight(_, _, _) => {
                Expression::ShiftRight(*loc, Box::new(assign), Box::new(set), ty.signed())
            }
            ast::Expression::AssignDivide(_, _, _) => {
                if ty.signed() {
                    Expression::SDivide(*loc, Box::new(assign), Box::new(set))
                } else {
                    Expression::UDivide(*loc, Box::new(assign), Box::new(set))
                }
            }
            ast::Expression::AssignModulo(_, _, _) => {
                if ty.signed() {
                    Expression::SModulo(*loc, Box::new(assign), Box::new(set))
                } else {
                    Expression::UModulo(*loc, Box::new(assign), Box::new(set))
                }
            }
            _ => unreachable!(),
        })
    };

    // either it's a variable, or a reference to an array element
    match var {
        ast::Expression::Variable(id) => {
            let tab = match vartab {
                &mut Some(ref mut tab) => tab,
                None => {
                    errors.push(Output::error(
                        *loc,
                        "cannot assign in constant expression".to_string(),
                    ));
                    return Err(());
                }
            };

            let v = tab.find(id, contract_no.unwrap(), ns, errors)?;

            match v.ty {
                resolver::Type::Bytes(_) | resolver::Type::Int(_) | resolver::Type::Uint(_) => (),
                _ => {
                    errors.push(Output::error(
                        var.loc(),
                        format!(
                            "variable ‘{}’ of incorrect type {}",
                            id.name.to_string(),
                            v.ty.to_string(ns)
                        ),
                    ));
                    return Err(());
                }
            };

            let lvalue = match &v.storage {
                Storage::Contract(n) => Expression::StorageLoad(
                    *loc,
                    v.ty.clone(),
                    Box::new(Expression::NumberLiteral(*loc, 256, n.clone())),
                ),
                Storage::Constant(_) => {
                    errors.push(Output::error(
                        *loc,
                        format!("cannot assign to constant ‘{}’", id.name),
                    ));
                    return Err(());
                }
                Storage::Local => Expression::Variable(id.loc, v.pos),
            };

            let set = op(lvalue, &v.ty, errors)?;

            cfg.add(
                tab,
                Instr::Set {
                    res: v.pos,
                    expr: set,
                },
            );

            match &v.storage {
                Storage::Contract(n) => {
                    cfg.writes_contract_storage = true;
                    cfg.add(
                        tab,
                        Instr::SetStorage {
                            ty: v.ty.clone(),
                            local: v.pos,
                            storage: Expression::NumberLiteral(*loc, 256, n.clone()),
                        },
                    );
                }
                Storage::Constant(_) => {
                    errors.push(Output::error(
                        *loc,
                        format!("cannot assign to constant ‘{}’", id.name),
                    ));
                    return Err(());
                }
                Storage::Local => {
                    // nothing to do
                }
            }

            Ok((Expression::Variable(id.loc, v.pos), v.ty))
        }
        _ => {
            let (var_expr, var_ty) = expression(var, cfg, contract_no, ns, vartab, errors)?;

            let tab = match vartab {
                &mut Some(ref mut tab) => tab,
                None => {
                    errors.push(Output::error(
                        *loc,
                        "cannot assign in constant expression".to_string(),
                    ));
                    return Err(());
                }
            };
            let pos = tab.temp_anonymous(&var_ty);

            match var_ty {
                resolver::Type::Ref(ref r_ty) => match r_ty.as_ref() {
                    resolver::Type::Bytes(_) | resolver::Type::Int(_) | resolver::Type::Uint(_) => {
                        let set = op(
                            cast(
                                loc,
                                var_expr.clone(),
                                &var_ty,
                                r_ty.as_ref(),
                                true,
                                ns,
                                errors,
                            )?,
                            &*r_ty,
                            errors,
                        )?;

                        cfg.add(
                            tab,
                            Instr::Set {
                                res: pos,
                                expr: set,
                            },
                        );
                        cfg.add(
                            tab,
                            Instr::Store {
                                dest: var_expr,
                                pos,
                            },
                        );
                        Ok((Expression::Variable(*loc, pos), r_ty.as_ref().clone()))
                    }
                    _ => {
                        errors.push(Output::error(
                            var.loc(),
                            format!("assigning to incorrect type {}", r_ty.to_string(ns)),
                        ));
                        Err(())
                    }
                },
                resolver::Type::StorageRef(ref r_ty) => match r_ty.as_ref() {
                    resolver::Type::Bytes(_) | resolver::Type::Int(_) | resolver::Type::Uint(_) => {
                        let set = op(
                            cast(
                                loc,
                                var_expr.clone(),
                                &var_ty,
                                r_ty.as_ref(),
                                true,
                                ns,
                                errors,
                            )?,
                            &*r_ty,
                            errors,
                        )?;

                        cfg.add(
                            tab,
                            Instr::Set {
                                res: pos,
                                expr: set,
                            },
                        );

                        if let Expression::StorageBytesSubscript(_, array, index) = var_expr {
                            // Set a byte in a byte array
                            cfg.add(
                                tab,
                                Instr::SetStorageBytes {
                                    local: pos,
                                    storage: array,
                                    offset: index,
                                },
                            );
                        } else {
                            // The value of the var_expr should be storage offset
                            cfg.add(
                                tab,
                                Instr::SetStorage {
                                    ty: *r_ty.clone(),
                                    local: pos,
                                    storage: var_expr,
                                },
                            );
                        }
                        cfg.writes_contract_storage = true;
                        Ok((Expression::Variable(*loc, pos), r_ty.as_ref().clone()))
                    }
                    _ => {
                        errors.push(Output::error(
                            var.loc(),
                            format!("assigning to incorrect type {}", r_ty.to_string(ns)),
                        ));
                        Err(())
                    }
                },
                _ => {
                    errors.push(Output::error(
                        var.loc(),
                        "expression is not assignable".to_string(),
                    ));
                    Err(())
                }
            }
        }
    }
}

/// Resolve an array subscript expression
fn array_subscript(
    loc: &ast::Loc,
    array: &ast::Expression,
    index: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let (mut array_expr, array_ty) = expression(array, cfg, contract_no, ns, vartab, errors)?;

    if array_ty.is_mapping() {
        return mapping_subscript(
            loc,
            array_expr,
            &array_ty,
            index,
            cfg,
            contract_no,
            ns,
            vartab,
            errors,
        );
    }

    let (index_expr, index_ty) = expression(index, cfg, contract_no, ns, vartab, errors)?;

    let tab = match vartab {
        &mut Some(ref mut tab) => tab,
        None => {
            errors.push(Output::error(
                *loc,
                "cannot read subscript in constant expression".to_string(),
            ));
            return Err(());
        }
    };

    let index_width = match index_ty {
        resolver::Type::Uint(w) => w,
        _ => {
            errors.push(Output::error(
                *loc,
                format!(
                    "array subscript must be an unsigned integer, not ‘{}’",
                    index_ty.to_string(ns)
                ),
            ));
            return Err(());
        }
    };

    if array_ty.is_storage_bytes() {
        return Ok((
            Expression::StorageBytesSubscript(
                *loc,
                Box::new(array_expr),
                Box::new(cast(
                    &index.loc(),
                    index_expr,
                    &index_ty,
                    &resolver::Type::Uint(32),
                    false,
                    ns,
                    errors,
                )?),
            ),
            resolver::Type::StorageRef(Box::new(resolver::Type::Bytes(1))),
        ));
    }

    let (array_length, array_length_ty) = match array_ty.deref() {
        resolver::Type::Bytes(n) => bigint_to_expression(loc, &BigInt::from(*n), errors)?,
        resolver::Type::Array(_, _) => match array_ty.array_length() {
            None => {
                if let resolver::Type::StorageRef(_) = array_ty {
                    let array_length = Expression::StorageLoad(
                        *loc,
                        resolver::Type::Uint(256),
                        Box::new(array_expr.clone()),
                    );

                    let slot_ty = resolver::Type::Uint(256);

                    array_expr = Expression::Keccak256(*loc, vec![(array_expr, slot_ty)]);

                    (array_length, resolver::Type::Uint(256))
                } else {
                    (
                        Expression::DynamicArrayLength(
                            *loc,
                            Box::new(cast(
                                &array.loc(),
                                array_expr.clone(),
                                &array_ty,
                                &array_ty.deref(),
                                true,
                                ns,
                                errors,
                            )?),
                        ),
                        resolver::Type::Uint(32),
                    )
                }
            }
            Some(l) => bigint_to_expression(loc, l, errors)?,
        },
        resolver::Type::String => {
            errors.push(Output::error(
                array.loc(),
                "array subscript is not permitted on string".to_string(),
            ));
            return Err(());
        }
        resolver::Type::DynamicBytes => (
            // FIXME does not handle bytes in storage
            Expression::DynamicArrayLength(*loc, Box::new(array_expr.clone())),
            resolver::Type::Uint(32),
        ),
        _ => {
            errors.push(Output::error(
                array.loc(),
                "expression is not an array".to_string(),
            ));
            return Err(());
        }
    };

    let array_width = array_length_ty.bits(ns);
    let width = std::cmp::max(array_width, index_width);
    let coerced_ty = resolver::Type::Uint(width);

    let pos = tab.temp(
        &ast::Identifier {
            name: "index".to_owned(),
            loc: *loc,
        },
        &coerced_ty,
    );

    cfg.add(
        tab,
        Instr::Set {
            res: pos,
            expr: cast(
                &index.loc(),
                index_expr,
                &index_ty,
                &coerced_ty,
                false,
                ns,
                errors,
            )?,
        },
    );

    // If the array is fixed length and the index also constant, the
    // branch will be optimized away.
    let out_of_bounds = cfg.new_basic_block("out_of_bounds".to_string());
    let in_bounds = cfg.new_basic_block("in_bounds".to_string());

    cfg.add(
        tab,
        Instr::BranchCond {
            cond: Expression::UMoreEqual(
                *loc,
                Box::new(Expression::Variable(index.loc(), pos)),
                Box::new(cast(
                    &array.loc(),
                    array_length.clone(),
                    &array_length_ty,
                    &coerced_ty,
                    false,
                    ns,
                    errors,
                )?),
            ),
            true_: out_of_bounds,
            false_: in_bounds,
        },
    );

    cfg.set_basic_block(out_of_bounds);
    cfg.add(tab, Instr::AssertFailure { expr: None });

    cfg.set_basic_block(in_bounds);

    if let resolver::Type::StorageRef(ty) = array_ty {
        let elem_ty = ty.storage_deref();
        let elem_size = elem_ty.storage_slots(ns);
        let mut nullsink = Vec::new();

        if let Ok(array_length) = eval_number_expression(&array_length, &mut nullsink) {
            if array_length.1.mul(elem_size.clone()).to_u64().is_some() {
                // we need to calculate the storage offset. If this can be done with 64 bit
                // arithmetic it will be much more efficient on wasm
                return Ok((
                    Expression::Add(
                        *loc,
                        Box::new(array_expr),
                        Box::new(Expression::ZeroExt(
                            *loc,
                            resolver::Type::Uint(256),
                            Box::new(Expression::Multiply(
                                *loc,
                                Box::new(cast(
                                    &index.loc(),
                                    Expression::Variable(index.loc(), pos),
                                    &coerced_ty,
                                    &resolver::Type::Uint(64),
                                    false,
                                    ns,
                                    errors,
                                )?),
                                Box::new(Expression::NumberLiteral(*loc, 64, elem_size)),
                            )),
                        )),
                    ),
                    elem_ty,
                ));
            }
        }

        Ok((
            array_offset(
                loc,
                array_expr,
                cast(
                    &index.loc(),
                    Expression::Variable(index.loc(), pos),
                    &coerced_ty,
                    &resolver::Type::Uint(256),
                    false,
                    ns,
                    errors,
                )?,
                elem_ty.clone(),
                ns,
            ),
            elem_ty,
        ))
    } else {
        match array_ty.deref() {
            resolver::Type::Bytes(array_length) => {
                let res_ty = resolver::Type::Bytes(1);

                Ok((
                    Expression::Trunc(
                        *loc,
                        res_ty.clone(),
                        Box::new(Expression::ShiftRight(
                            *loc,
                            Box::new(array_expr),
                            // shift by (array_length - 1 - index) * 8
                            Box::new(Expression::ShiftLeft(
                                *loc,
                                Box::new(Expression::Subtract(
                                    *loc,
                                    Box::new(Expression::NumberLiteral(
                                        *loc,
                                        *array_length as u16 * 8,
                                        BigInt::from_u8(array_length - 1).unwrap(),
                                    )),
                                    Box::new(cast_shift_arg(
                                        loc,
                                        Expression::Variable(index.loc(), pos),
                                        index_width,
                                        &array_ty,
                                        ns,
                                    )),
                                )),
                                Box::new(Expression::NumberLiteral(
                                    *loc,
                                    *array_length as u16 * 8,
                                    BigInt::from_u8(3).unwrap(),
                                )),
                            )),
                            false,
                        )),
                    ),
                    res_ty,
                ))
            }
            resolver::Type::Array(_, dim) if dim.last().unwrap().is_some() => Ok((
                Expression::ArraySubscript(
                    *loc,
                    Box::new(cast(
                        &array.loc(),
                        array_expr,
                        &array_ty,
                        &array_ty.deref(),
                        true,
                        ns,
                        errors,
                    )?),
                    Box::new(Expression::Variable(index.loc(), pos)),
                ),
                array_ty.array_deref(),
            )),
            resolver::Type::DynamicBytes | resolver::Type::Array(_, _) => Ok((
                Expression::DynamicArraySubscript(
                    *loc,
                    Box::new(cast(
                        &array.loc(),
                        array_expr,
                        &array_ty,
                        &array_ty.deref(),
                        true,
                        ns,
                        errors,
                    )?),
                    array_ty.array_deref(),
                    Box::new(Expression::Variable(index.loc(), pos)),
                ),
                array_ty.array_deref(),
            )),
            _ => {
                // should not happen as type-checking already done
                unreachable!();
            }
        }
    }
}

/// Resolve a function call with positional arguments
fn struct_literal(
    loc: &ast::Loc,
    struct_no: usize,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let struct_def = &ns.structs[struct_no];

    if args.len() != struct_def.fields.len() {
        errors.push(Output::error(
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
            let (expr, expr_type) = expression(&a, cfg, contract_no, ns, vartab, errors)?;

            fields.push(cast(
                loc,
                expr,
                &expr_type,
                &struct_def.fields[i].ty,
                true,
                ns,
                errors,
            )?);
        }

        let ty = resolver::Type::Struct(struct_no);

        Ok((Expression::StructLiteral(*loc, ty.clone(), fields), ty))
    }
}

/// Resolve a function call with positional arguments
fn function_call_pos_args(
    loc: &ast::Loc,
    id: &ast::Identifier,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    // Try to resolve as a function call
    let funcs = ns.resolve_func(contract_no.unwrap(), &id, errors)?;

    let mut resolved_args = Vec::new();
    let mut resolved_types = Vec::new();

    for arg in args {
        let (expr, expr_type) = expression(arg, cfg, contract_no, ns, vartab, errors)?;

        resolved_args.push(Box::new(expr));
        resolved_types.push(expr_type);
    }

    let mut temp_errors = Vec::new();

    // function call
    for f in funcs {
        let func = &ns.contracts[contract_no.unwrap()].functions[f.1];

        if func.params.len() != args.len() {
            temp_errors.push(Output::error(
                *loc,
                format!(
                    "function expects {} arguments, {} provided",
                    func.params.len(),
                    args.len()
                ),
            ));
            continue;
        }

        let mut matches = true;
        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for (i, param) in func.params.iter().enumerate() {
            let arg = &resolved_args[i];

            match cast(
                &ast::Loc(0, 0),
                *arg.clone(),
                &resolved_types[i],
                &param.ty,
                true,
                ns,
                &mut temp_errors,
            ) {
                Ok(expr) => cast_args.push(expr),
                Err(()) => {
                    matches = false;
                    break;
                }
            }
        }

        if matches {
            return Ok((
                Expression::LocalFunctionCall(*loc, f.1, cast_args),
                resolver::Type::Undef,
            ));
        }
    }

    if funcs.len() == 1 {
        errors.append(&mut temp_errors);
    } else {
        errors.push(Output::error(
            *loc,
            "cannot find overloaded function which matches signature".to_string(),
        ));
    }

    Err(())
}

/// Resolve a function call with named arguments
fn function_call_with_named_args(
    loc: &ast::Loc,
    id: &ast::Identifier,
    args: &[ast::NamedArgument],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    // Try to resolve as a function call
    let funcs = ns.resolve_func(contract_no.unwrap(), &id, errors)?;

    let mut arguments = HashMap::new();

    for arg in args {
        if arguments.contains_key(&arg.name.name) {
            errors.push(Output::error(
                arg.name.loc,
                format!("duplicate argument with name ‘{}’", arg.name.name),
            ));
            return Err(());
        }
        arguments.insert(
            arg.name.name.to_string(),
            expression(&arg.expr, cfg, contract_no, ns, vartab, errors)?,
        );
    }

    let mut temp_errors = Vec::new();

    // function call
    for f in funcs {
        let func = &ns.contracts[contract_no.unwrap()].functions[f.1];

        if func.params.len() != args.len() {
            temp_errors.push(Output::error(
                *loc,
                format!(
                    "function expects {} arguments, {} provided",
                    func.params.len(),
                    args.len()
                ),
            ));
            continue;
        }

        let mut matches = true;
        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for param in func.params.iter() {
            let arg = match arguments.get(&param.name) {
                Some(a) => a,
                None => {
                    matches = false;
                    temp_errors.push(Output::error(
                        *loc,
                        format!(
                            "missing argument ‘{}’ to function ‘{}’",
                            param.name, func.name,
                        ),
                    ));
                    break;
                }
            };

            match cast(
                &ast::Loc(0, 0),
                arg.0.clone(),
                &arg.1,
                &param.ty,
                true,
                ns,
                &mut temp_errors,
            ) {
                Ok(expr) => cast_args.push(expr),
                Err(()) => {
                    matches = false;
                    break;
                }
            }
        }

        if matches {
            return Ok((
                Expression::LocalFunctionCall(*loc, f.1, cast_args),
                resolver::Type::Undef,
            ));
        }
    }

    if funcs.len() == 1 {
        errors.append(&mut temp_errors);
    } else {
        errors.push(Output::error(
            *loc,
            "cannot find overloaded function which matches signature".to_string(),
        ));
    }

    Err(())
}

/// Resolve a struct literal with named fields
fn named_struct_literal(
    loc: &ast::Loc,
    struct_no: usize,
    args: &[ast::NamedArgument],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let struct_def = &ns.structs[struct_no];

    if args.len() != struct_def.fields.len() {
        errors.push(Output::error(
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
                    let (expr, expr_type) =
                        expression(&a.expr, cfg, contract_no, ns, vartab, errors)?;

                    fields[i] = cast(loc, expr, &expr_type, &f.ty, true, ns, errors)?;
                }
                None => {
                    errors.push(Output::error(
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
        let ty = resolver::Type::Struct(struct_no);
        Ok((Expression::StructLiteral(*loc, ty.clone(), fields), ty))
    }
}

/// Resolve a method call with positional arguments
fn method_call_pos_args(
    loc: &ast::Loc,
    var: &ast::Expression,
    func: &ast::Identifier,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let (var_expr, var_ty) = expression(var, cfg, contract_no, ns, vartab, errors)?;

    if let resolver::Type::StorageRef(ty) = &var_ty {
        match ty.as_ref() {
            resolver::Type::Array(_, dim) => {
                if func.name == "push" {
                    return if dim.last().unwrap().is_some() {
                        errors.push(Output::error(
                            func.loc,
                            "method ‘push()’ not allowed on fixed length array".to_string(),
                        ));
                        Err(())
                    } else {
                        array_push(
                            loc,
                            var_expr,
                            func,
                            ty,
                            args,
                            cfg,
                            contract_no,
                            ns,
                            vartab,
                            errors,
                        )
                    };
                }
                if func.name == "pop" {
                    return if dim.last().unwrap().is_some() {
                        errors.push(Output::error(
                            func.loc,
                            "method ‘pop()’ not allowed on fixed length array".to_string(),
                        ));
                        Err(())
                    } else {
                        array_pop(loc, var_expr, func, ty, args, cfg, ns, vartab, errors)
                    };
                }
            }
            resolver::Type::DynamicBytes => match func.name.as_str() {
                "push" => {
                    return bytes_push(
                        loc,
                        var_expr,
                        func,
                        args,
                        cfg,
                        contract_no,
                        ns,
                        vartab,
                        errors,
                    );
                }
                "pop" => {
                    return bytes_pop(loc, var_expr, func, args, cfg, errors);
                }
                _ => {}
            },
            _ => {}
        }
    }

    if let resolver::Type::Contract(contract_no) = &var_ty.deref() {
        let mut resolved_args = Vec::new();
        let mut resolved_types = Vec::new();

        for arg in args {
            let (expr, expr_type) = expression(arg, cfg, Some(*contract_no), ns, vartab, errors)?;
            resolved_args.push(Box::new(expr));
            resolved_types.push(expr_type);
        }

        let mut temp_errors = Vec::new();

        let mut name_match = 0;
        for (n, ftype) in ns.contracts[*contract_no].functions.iter().enumerate() {
            if func.name != ftype.name {
                continue;
            }

            name_match += 1;

            if ftype.params.len() != args.len() {
                continue;
            }

            if ftype.params.len() != args.len() {
                temp_errors.push(Output::error(
                    *loc,
                    format!(
                        "function expects {} arguments, {} provided",
                        ftype.params.len(),
                        args.len()
                    ),
                ));
                continue;
            }
            let mut matches = true;
            let mut cast_args = Vec::new();
            // check if arguments can be implicitly casted
            for (i, param) in ftype.params.iter().enumerate() {
                let arg = &resolved_args[i];
                match cast(
                    &ast::Loc(0, 0),
                    *arg.clone(),
                    &resolved_types[i],
                    &param.ty,
                    true,
                    ns,
                    &mut temp_errors,
                ) {
                    Ok(expr) => cast_args.push(expr),
                    Err(()) => {
                        matches = false;
                        break;
                    }
                }
            }
            if matches {
                return Ok((
                    Expression::ExternalFunctionCall(
                        *loc,
                        *contract_no,
                        n,
                        Box::new(cast(
                            &var.loc(),
                            var_expr,
                            &var_ty,
                            // FIXME: make payable if function is payable
                            &resolver::Type::Address(false),
                            true,
                            ns,
                            errors,
                        )?),
                        cast_args,
                    ),
                    resolver::Type::Undef,
                ));
            }
        }

        if name_match == 1 {
            errors.append(&mut temp_errors);
        } else {
            errors.push(Output::error(
                *loc,
                "cannot find overloaded function which matches signature".to_string(),
            ));
        }

        return Err(());
    }

    errors.push(Output::error(
        func.loc,
        format!("method ‘{}’ does not exist", func.name),
    ));

    Err(())
}

fn method_call_with_named_args(
    loc: &ast::Loc,
    var: &ast::Expression,
    func_name: &ast::Identifier,
    args: &[ast::NamedArgument],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let (var_expr, var_ty) = expression(var, cfg, contract_no, ns, vartab, errors)?;

    if let resolver::Type::Contract(external_contract_no) = &var_ty.deref() {
        let mut arguments = HashMap::new();

        for arg in args {
            if arguments.contains_key(&arg.name.name) {
                errors.push(Output::error(
                    arg.name.loc,
                    format!("duplicate argument with name ‘{}’", arg.name.name),
                ));
                return Err(());
            }
            arguments.insert(
                arg.name.name.to_string(),
                expression(&arg.expr, cfg, contract_no, ns, vartab, errors)?,
            );
        }

        let mut temp_errors = Vec::new();

        let mut name_match = 0;

        // function call
        for (func_no, func) in ns.contracts[*external_contract_no]
            .functions
            .iter()
            .enumerate()
        {
            if func.name != func_name.name {
                continue;
            }

            name_match += 1;

            if func.params.len() != args.len() {
                temp_errors.push(Output::error(
                    *loc,
                    format!(
                        "function expects {} arguments, {} provided",
                        func.params.len(),
                        args.len()
                    ),
                ));
                continue;
            }
            let mut matches = true;
            let mut cast_args = Vec::new();
            // check if arguments can be implicitly casted
            for param in func.params.iter() {
                let arg = match arguments.get(&param.name) {
                    Some(a) => a,
                    None => {
                        matches = false;
                        temp_errors.push(Output::error(
                            *loc,
                            format!(
                                "missing argument ‘{}’ to function ‘{}’",
                                param.name, func.name,
                            ),
                        ));
                        break;
                    }
                };
                match cast(
                    &ast::Loc(0, 0),
                    arg.0.clone(),
                    &arg.1,
                    &param.ty,
                    true,
                    ns,
                    &mut temp_errors,
                ) {
                    Ok(expr) => cast_args.push(expr),
                    Err(()) => {
                        matches = false;
                        break;
                    }
                }
            }

            if matches {
                return Ok((
                    Expression::ExternalFunctionCall(
                        *loc,
                        *external_contract_no,
                        func_no,
                        Box::new(cast(
                            &var.loc(),
                            var_expr,
                            &var_ty,
                            // FIXME: make payable if function is payable
                            &resolver::Type::Address(false),
                            true,
                            ns,
                            errors,
                        )?),
                        cast_args,
                    ),
                    resolver::Type::Undef,
                ));
            }
        }

        match name_match {
            0 => {
                errors.push(Output::error(
                    *loc,
                    format!(
                        "contract ‘{}’ does not have function ‘{}’",
                        var_ty.deref().to_string(ns),
                        func_name.name
                    ),
                ));
            }
            1 => {
                errors.append(&mut temp_errors);
            }
            _ => {
                errors.push(Output::error(
                    *loc,
                    "cannot find overloaded function which matches signature".to_string(),
                ));
            }
        }
        return Err(());
    }

    errors.push(Output::error(
        func_name.loc,
        format!("method ‘{}’ does not exist", func_name.name),
    ));

    Err(())
}

// When generating shifts, llvm wants both arguments to have the same width. We want the
// result of the shift to be left argument, so this function coercies the right argument
// into the right length.
fn cast_shift_arg(
    loc: &ast::Loc,
    expr: Expression,
    from_width: u16,
    ty: &resolver::Type,
    ns: &resolver::Namespace,
) -> Expression {
    let to_width = ty.bits(ns);

    if from_width == to_width {
        expr
    } else if from_width < to_width && ty.signed() {
        Expression::SignExt(*loc, ty.clone(), Box::new(expr))
    } else if from_width < to_width && !ty.signed() {
        Expression::ZeroExt(*loc, ty.clone(), Box::new(expr))
    } else {
        Expression::Trunc(*loc, ty.clone(), Box::new(expr))
    }
}

/// Given an parsed literal array, ensure that it is valid. All the elements in the array
/// must of the same type. The array might be a multidimensional array; all the leaf nodes
/// must match.
fn resolve_array_literal(
    loc: &ast::Loc,
    exprs: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let mut dims = Box::new(Vec::new());
    let mut flattened = Vec::new();

    check_subarrays(exprs, &mut Some(&mut dims), &mut flattened, errors)?;

    if flattened.is_empty() {
        errors.push(Output::error(
            *loc,
            "array requires at least one element".to_string(),
        ));
        return Err(());
    }

    let mut flattened = flattened.iter();

    // We follow the solidity scheme were everthing gets implicitly converted to the
    // type of the first element
    let (first, ty) = expression(
        flattened.next().unwrap(),
        cfg,
        contract_no,
        ns,
        vartab,
        errors,
    )?;

    let mut exprs = vec![first];

    for e in flattened {
        let (mut other, oty) = expression(e, cfg, contract_no, ns, vartab, errors)?;

        if oty != ty {
            other = cast(&e.loc(), other, &oty, &ty, true, ns, errors)?;
        }

        exprs.push(other);
    }

    let aty = resolver::Type::Array(
        Box::new(ty),
        dims.iter()
            .map(|n| Some(BigInt::from_u32(*n).unwrap()))
            .collect::<Vec<Option<BigInt>>>(),
    );

    if vartab.is_none() {
        Ok((Expression::ConstArrayLiteral(*loc, *dims, exprs), aty))
    } else {
        Ok((
            Expression::ArrayLiteral(*loc, aty.clone(), *dims, exprs),
            aty,
        ))
    }
}

/// Traverse the literal looking for sub arrays. Ensure that all the sub
/// arrays are the same length, and returned a flattened array of elements
fn check_subarrays<'a>(
    exprs: &'a [ast::Expression],
    dims: &mut Option<&mut Vec<u32>>,
    flatten: &mut Vec<&'a ast::Expression>,
    errors: &mut Vec<output::Output>,
) -> Result<(), ()> {
    if let Some(ast::Expression::ArrayLiteral(_, first)) = exprs.get(0) {
        // ensure all elements are array literals of the same length
        check_subarrays(first, dims, flatten, errors)?;

        for (i, e) in exprs.iter().enumerate().skip(1) {
            if let ast::Expression::ArrayLiteral(_, other) = e {
                if other.len() != first.len() {
                    errors.push(Output::error(
                        e.loc(),
                        format!(
                            "array elements should be identical, sub array {} has {} elements rather than {}", i + 1, other.len(), first.len()
                        ),
                    ));
                    return Err(());
                }
                check_subarrays(other, &mut None, flatten, errors)?;
            } else {
                errors.push(Output::error(
                    e.loc(),
                    format!("array element {} should also be an array", i + 1),
                ));
                return Err(());
            }
        }
    } else {
        for (i, e) in exprs.iter().enumerate().skip(1) {
            if let ast::Expression::ArrayLiteral(loc, _) = e {
                errors.push(Output::error(
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

/// The parser generates parameter lists for lists. Sometimes this needs to be a
/// simple expression list.
pub fn parameter_list_to_expr_list<'a>(
    e: &'a ast::Expression,
    errors: &mut Vec<output::Output>,
) -> Result<Vec<&'a ast::Expression>, ()> {
    if let ast::Expression::List(_, v) = &e {
        let mut list = Vec::new();
        let mut broken = false;

        for e in v {
            match &e.1 {
                None => {
                    errors.push(Output::error(e.0, "stray comma".to_string()));
                    broken = true;
                }
                Some(ast::Parameter {
                    name: Some(name), ..
                }) => {
                    errors.push(Output::error(name.loc, "single value expected".to_string()));
                    broken = true;
                }
                Some(ast::Parameter {
                    storage: Some(storage),
                    ..
                }) => {
                    errors.push(Output::error(
                        *storage.loc(),
                        "storage specified not permitted here".to_string(),
                    ));
                    broken = true;
                }
                Some(ast::Parameter { ty, .. }) => {
                    list.push(ty);
                }
            }
        }

        if !broken {
            Ok(list)
        } else {
            Err(())
        }
    } else {
        Ok(vec![e])
    }
}

/// Resolve function call
pub fn function_call_expr(
    loc: &ast::Loc,
    ty: &ast::Expression,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<Output>,
) -> Result<(Expression, resolver::Type), ()> {
    match ty {
        ast::Expression::MemberAccess(_, member, func) => method_call_pos_args(
            loc,
            member,
            func,
            args,
            cfg,
            contract_no,
            ns,
            vartab,
            errors,
        ),
        ast::Expression::Variable(id) => {
            function_call_pos_args(loc, &id, args, cfg, contract_no, ns, vartab, errors)
        }
        ast::Expression::ArraySubscript(_, _, _) => {
            errors.push(Output::error(ty.loc(), "unexpected array type".to_string()));
            Err(())
        }
        _ => {
            errors.push(Output::error(
                ty.loc(),
                "expression not expected here".to_string(),
            ));
            Err(())
        }
    }
}

/// Resolve function call expression with named arguments
pub fn named_function_call_expr(
    loc: &ast::Loc,
    ty: &ast::Expression,
    args: &[ast::NamedArgument],
    cfg: &mut ControlFlowGraph,
    contract_no: Option<usize>,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<Output>,
) -> Result<(Expression, resolver::Type), ()> {
    match ty {
        ast::Expression::MemberAccess(_, member, func) => method_call_with_named_args(
            loc,
            member,
            func,
            args,
            cfg,
            contract_no,
            ns,
            vartab,
            errors,
        ),
        ast::Expression::Variable(id) => {
            function_call_with_named_args(loc, &id, args, cfg, contract_no, ns, vartab, errors)
        }
        ast::Expression::ArraySubscript(_, _, _) => {
            errors.push(Output::error(ty.loc(), "unexpected array type".to_string()));
            Err(())
        }
        _ => {
            errors.push(Output::error(
                ty.loc(),
                "expression not expected here".to_string(),
            ));
            Err(())
        }
    }
}

/// Convert a function call expression to CFG in expression context
fn emit_function_call(
    expr: Expression,
    expr_ty: resolver::Type,
    contract_no: usize,
    cfg: &mut ControlFlowGraph,
    ns: &resolver::Namespace,
    vartab: &mut Option<&mut Vartable>,
) -> Vec<(Expression, resolver::Type)> {
    let tab = match vartab {
        &mut Some(ref mut tab) => tab,
        None => unreachable!(),
    };

    match expr {
        Expression::LocalFunctionCall(_, func, args) => {
            let ftype = &ns.contracts[contract_no].functions[func];

            if !ftype.returns.is_empty() {
                let mut res = Vec::new();
                let mut returns = Vec::new();

                for ret in &ftype.returns {
                    let id = ast::Identifier {
                        loc: ast::Loc(0, 0),
                        name: ret.name.to_owned(),
                    };

                    let temp_pos = tab.temp(&id, &ret.ty);
                    res.push(temp_pos);
                    returns.push((Expression::Variable(id.loc, temp_pos), ret.ty.clone()));
                }

                cfg.add(tab, Instr::Call { res, func, args });

                returns
            } else {
                cfg.add(
                    tab,
                    Instr::Call {
                        res: Vec::new(),
                        func,
                        args,
                    },
                );

                vec![(
                    if ftype.noreturn {
                        Expression::Unreachable
                    } else {
                        Expression::Poison
                    },
                    resolver::Type::Undef,
                )]
            }
        }
        Expression::ExternalFunctionCall(loc, contract_no, function_no, address, args) => {
            let ftype = &ns.contracts[contract_no].functions[function_no];

            cfg.add(
                tab,
                Instr::ExternalCall {
                    success: None,
                    address: *address,
                    contract_no,
                    function_no,
                    args,
                },
            );

            if !ftype.returns.is_empty() {
                let mut returns = Vec::new();
                let mut res = Vec::new();

                for ret in &ftype.returns {
                    let id = ast::Identifier {
                        loc: ast::Loc(0, 0),
                        name: "".to_owned(),
                    };
                    let temp_pos = tab.temp(&id, &ret.ty);
                    res.push(temp_pos);
                    returns.push((Expression::Variable(id.loc, temp_pos), ret.ty.clone()));
                }

                cfg.add(
                    tab,
                    Instr::AbiDecode {
                        res,
                        selector: None,
                        exception: None,
                        tys: ftype.returns.clone(),
                        data: Expression::ReturnData(loc),
                    },
                );

                returns
            } else {
                vec![(Expression::Poison, resolver::Type::Undef)]
            }
        }
        _ => vec![(expr, expr_ty)],
    }
}

/// Convert a constructor call expression to CFG in expression context
fn emit_constructor_call(
    expr: Expression,
    expr_ty: resolver::Type,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Option<&mut Vartable>,
) -> (Expression, resolver::Type) {
    let tab = match vartab {
        &mut Some(ref mut tab) => tab,
        None => unreachable!(),
    };

    match expr {
        Expression::Constructor(loc, contract_no, constructor_no, args) => {
            let address_res = tab.temp_anonymous(&resolver::Type::Contract(contract_no));

            cfg.add(
                tab,
                Instr::Constructor {
                    success: None,
                    res: address_res,
                    contract_no,
                    constructor_no,
                    args,
                },
            );

            (
                Expression::Variable(loc, address_res),
                resolver::Type::Contract(contract_no),
            )
        }
        _ => (expr, expr_ty),
    }
}
