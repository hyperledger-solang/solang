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
use unescape::unescape;

use hex;
use output;
use output::Output;
use parser::ast;
use parser::ast::Loc;
use resolver;
use resolver::address::to_hexstr_eip55;
use resolver::cfg::{ControlFlowGraph, Instr, Storage, Vartable};
use resolver::eval::eval_number_expression;
use resolver::storage::array_offset;

#[derive(PartialEq, Clone, Debug)]
pub enum Expression {
    BoolLiteral(Loc, bool),
    BytesLiteral(Loc, Vec<u8>),
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

    AllocDynamicArray(Loc, resolver::Type, Box<Expression>),
    DynamicArrayLength(Loc, Box<Expression>),
    DynamicArraySubscript(Loc, Box<Expression>, resolver::Type, Box<Expression>),

    Or(Loc, Box<Expression>, Box<Expression>),
    And(Loc, Box<Expression>, Box<Expression>),

    Keccak256(Loc, Box<Expression>),

    Poison,
}

impl Expression {
    /// Return the location for this expression
    pub fn loc(&self) -> Loc {
        match self {
            Expression::BoolLiteral(loc, _)
            | Expression::BytesLiteral(loc, _)
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
            | Expression::AllocDynamicArray(loc, _, _)
            | Expression::DynamicArrayLength(loc, _)
            | Expression::DynamicArraySubscript(loc, _, _, _)
            | Expression::Keccak256(loc, _)
            | Expression::And(loc, _, _) => *loc,
            Expression::Poison => unreachable!(),
        }
    }
    /// Returns true if the Expression may load from contract storage using StorageLoad
    pub fn reads_contract_storage(&self) -> bool {
        match self {
            Expression::StorageLoad(_, _, _) => true,
            Expression::BoolLiteral(_, _)
            | Expression::BytesLiteral(_, _)
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
            Expression::AllocDynamicArray(_, _, s) => s.reads_contract_storage(),
            Expression::DynamicArrayLength(_, s) => s.reads_contract_storage(),
            Expression::StructMember(_, s, _) => s.reads_contract_storage(),
            Expression::Keccak256(_, e) => e.reads_contract_storage(),
            Expression::And(_, l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::Or(_, l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::Poison => false,
        }
    }
}

fn coerce(
    l: &resolver::Type,
    l_loc: &ast::Loc,
    r: &resolver::Type,
    r_loc: &ast::Loc,
    ns: &resolver::Contract,
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
    ns: &resolver::Contract,
    errors: &mut Vec<output::Output>,
) -> Result<(u16, bool), ()> {
    match l {
        resolver::Type::Primitive(ast::PrimitiveType::Uint(n)) => Ok((*n, false)),
        resolver::Type::Primitive(ast::PrimitiveType::Int(n)) => Ok((*n, true)),
        resolver::Type::Primitive(ast::PrimitiveType::Bytes(n)) if allow_bytes => {
            Ok((*n as u16 * 8, false))
        }
        resolver::Type::Primitive(t) => {
            errors.push(Output::error(
                *l_loc,
                format!("expression of type {} not allowed", t.to_string()),
            ));
            Err(())
        }
        resolver::Type::Enum(n) => {
            errors.push(Output::error(
                *l_loc,
                format!("type enum {}.{} not allowed", ns.name, ns.enums[*n].name),
            ));
            Err(())
        }
        resolver::Type::Struct(n) => {
            errors.push(Output::error(
                *l_loc,
                format!(
                    "type struct {}.{} not allowed",
                    ns.name, ns.structs[*n].name
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
    }
}

fn coerce_int(
    l: &resolver::Type,
    l_loc: &ast::Loc,
    r: &resolver::Type,
    r_loc: &ast::Loc,
    allow_bytes: bool,
    ns: &resolver::Contract,
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
        (
            resolver::Type::Primitive(ast::PrimitiveType::Bytes(left_length)),
            resolver::Type::Primitive(ast::PrimitiveType::Bytes(right_length)),
        ) if allow_bytes => {
            return Ok(resolver::Type::Primitive(ast::PrimitiveType::Bytes(
                std::cmp::max(*left_length, *right_length),
            )));
        }
        _ => (),
    }

    let (left_len, left_signed) = get_int_length(l, l_loc, false, ns, errors)?;

    let (right_len, right_signed) = get_int_length(r, r_loc, false, ns, errors)?;

    Ok(resolver::Type::Primitive(
        match (left_signed, right_signed) {
            (true, true) => ast::PrimitiveType::Int(cmp::max(left_len, right_len)),
            (false, false) => ast::PrimitiveType::Uint(cmp::max(left_len, right_len)),
            (true, false) => {
                ast::PrimitiveType::Int(cmp::max(left_len, cmp::min(right_len + 8, 256)))
            }
            (false, true) => {
                ast::PrimitiveType::Int(cmp::max(cmp::min(left_len + 8, 256), right_len))
            }
        },
    ))
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
                resolver::Type::Primitive(ast::PrimitiveType::Int(int_size)),
            ))
        }
    } else if bits > 256 {
        errors.push(Output::error(*loc, format!("{} is too large", n)));
        Err(())
    } else {
        Ok((
            Expression::NumberLiteral(*loc, int_size, n.clone()),
            resolver::Type::Primitive(ast::PrimitiveType::Uint(int_size)),
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
    ns: &resolver::Contract,
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

    if from == to {
        return Ok(expr);
    }

    let (from_conv, to_conv) = {
        if implicit {
            (from.clone(), to.clone())
        } else {
            let from_conv = if let resolver::Type::Enum(n) = from {
                resolver::Type::Primitive(ns.enums[*n].ty)
            } else {
                from.clone()
            };

            let to_conv = if let resolver::Type::Enum(n) = to {
                resolver::Type::Primitive(ns.enums[*n].ty)
            } else {
                to.clone()
            };

            (from_conv, to_conv)
        }
    };

    // Special case: when converting literal sign can change if it fits
    match (&expr, &from_conv, &to_conv) {
        (
            &Expression::NumberLiteral(_, _, ref n),
            &resolver::Type::Primitive(_),
            &resolver::Type::Primitive(ast::PrimitiveType::Uint(to_len)),
        ) => {
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
        (
            &Expression::NumberLiteral(_, _, ref n),
            &resolver::Type::Primitive(_),
            &resolver::Type::Primitive(ast::PrimitiveType::Int(to_len)),
        ) => {
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
        (
            &Expression::BytesLiteral(_, ref bs),
            &resolver::Type::Primitive(_),
            &resolver::Type::Primitive(ast::PrimitiveType::Bytes(to_len)),
        ) => {
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
        _ => (),
    };

    #[allow(clippy::comparison_chain)]
    match (from_conv, to_conv) {
        (
            resolver::Type::Primitive(ast::PrimitiveType::Uint(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Uint(to_len)),
        ) => match from_len.cmp(&to_len) {
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
        },
        (
            resolver::Type::Primitive(ast::PrimitiveType::Int(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Int(to_len)),
        ) => match from_len.cmp(&to_len) {
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
        (
            resolver::Type::Primitive(ast::PrimitiveType::Uint(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Int(to_len)),
        ) if to_len > from_len => Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr))),
        (
            resolver::Type::Primitive(ast::PrimitiveType::Int(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Uint(to_len)),
        ) => {
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
        (
            resolver::Type::Primitive(ast::PrimitiveType::Uint(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Int(to_len)),
        ) => {
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
        (
            resolver::Type::Primitive(ast::PrimitiveType::Uint(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Address),
        )
        | (
            resolver::Type::Primitive(ast::PrimitiveType::Int(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Address),
        ) => {
            if implicit {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "implicit conversion from {} to address not allowed",
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if from_len > 160 {
                Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
            } else if from_len < 160 {
                Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr)))
            } else {
                Ok(expr)
            }
        }
        // Casting int address to int
        (
            resolver::Type::Primitive(ast::PrimitiveType::Address),
            resolver::Type::Primitive(ast::PrimitiveType::Uint(to_len)),
        )
        | (
            resolver::Type::Primitive(ast::PrimitiveType::Address),
            resolver::Type::Primitive(ast::PrimitiveType::Int(to_len)),
        ) => {
            if implicit {
                errors.push(Output::type_error(
                    *loc,
                    format!(
                        "implicit conversion to {} from address not allowed",
                        from.to_string(ns)
                    ),
                ));
                Err(())
            } else if to_len < 160 {
                Ok(Expression::Trunc(*loc, to.clone(), Box::new(expr)))
            } else if to_len > 160 {
                Ok(Expression::ZeroExt(*loc, to.clone(), Box::new(expr)))
            } else {
                Ok(expr)
            }
        }
        // Lengthing or shorting a fixed bytes array
        (
            resolver::Type::Primitive(ast::PrimitiveType::Bytes(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Bytes(to_len)),
        ) => {
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
        (
            resolver::Type::Primitive(ast::PrimitiveType::Bytes(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Uint(to_len)),
        )
        | (
            resolver::Type::Primitive(ast::PrimitiveType::Bytes(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Int(to_len)),
        ) => {
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
        (
            resolver::Type::Primitive(ast::PrimitiveType::Uint(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Bytes(to_len)),
        )
        | (
            resolver::Type::Primitive(ast::PrimitiveType::Int(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Bytes(to_len)),
        ) => {
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
        (
            resolver::Type::Primitive(ast::PrimitiveType::Bytes(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Address),
        ) => {
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
            } else if from_len != 20 {
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
        (
            resolver::Type::Primitive(ast::PrimitiveType::Address),
            resolver::Type::Primitive(ast::PrimitiveType::Bytes(to_len)),
        ) => {
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
            } else if to_len != 20 {
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
        // string conversions
        (
            resolver::Type::Primitive(ast::PrimitiveType::Bytes(_)),
            resolver::Type::Primitive(ast::PrimitiveType::String),
        ) => Ok(expr),
        (
            resolver::Type::Primitive(ast::PrimitiveType::String),
            resolver::Type::Primitive(ast::PrimitiveType::Bytes(to_len)),
        ) => {
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
    ns: &resolver::Contract,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    match expr {
        ast::Expression::ArrayLiteral(loc, exprs) => {
            resolve_array_literal(loc, exprs, cfg, ns, vartab, errors)
        }
        ast::Expression::BoolLiteral(loc, v) => Ok((
            Expression::BoolLiteral(*loc, *v),
            resolver::Type::Primitive(ast::PrimitiveType::Bool),
        )),
        ast::Expression::StringLiteral(v) => {
            // Concatenate the strings
            let mut result = String::new();
            let mut loc = ast::Loc(0, 0);

            for s in v {
                // unescape supports octal escape values, solc does not
                // neither solc nor unescape support unicode code points like \u{61}
                match unescape(&s.string) {
                    Some(v) => {
                        result.push_str(&v);
                        if loc.0 == 0 {
                            loc.0 = s.loc.0;
                        }
                        loc.1 = s.loc.1;
                    }
                    None => {
                        // would be helpful if unescape told us what/where the problem was
                        errors.push(Output::error(
                            s.loc,
                            format!("string \"{}\" has invalid escape", s.string),
                        ));
                        return Err(());
                    }
                }
            }

            let length = result.len();

            Ok((
                Expression::BytesLiteral(loc, result.into_bytes()),
                resolver::Type::Primitive(ast::PrimitiveType::Bytes(length as u8)),
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
                resolver::Type::Primitive(ast::PrimitiveType::Bytes(length as u8)),
            ))
        }
        ast::Expression::NumberLiteral(loc, b) => bigint_to_expression(loc, b, errors),
        ast::Expression::AddressLiteral(loc, n) => {
            let address = to_hexstr_eip55(n);

            if address == *n {
                let s: String = address.chars().skip(2).collect();

                Ok((
                    Expression::NumberLiteral(*loc, 160, BigInt::from_str_radix(&s, 16).unwrap()),
                    resolver::Type::Primitive(ast::PrimitiveType::Address),
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
        }
        ast::Expression::Variable(id) => {
            if let Some(ref mut tab) = *vartab {
                let v = tab.find(id, ns, errors)?;
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
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

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
        ast::Expression::Subtract(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

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
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

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
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

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
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

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
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            // left hand side may be bytes/int/uint
            // right hand size may be int/uint
            let _ = get_int_length(&left_type, &l.loc(), true, ns, errors)?;
            let (right_length, _) = get_int_length(&right_type, &r.loc(), false, ns, errors)?;

            Ok((
                Expression::ShiftLeft(
                    *loc,
                    Box::new(left),
                    Box::new(cast_shift_arg(loc, right, right_length, &left_type)),
                ),
                left_type,
            ))
        }
        ast::Expression::ShiftRight(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            // left hand side may be bytes/int/uint
            // right hand size may be int/uint
            let _ = get_int_length(&left_type, &l.loc(), true, ns, errors)?;
            let (right_length, _) = get_int_length(&right_type, &r.loc(), false, ns, errors)?;

            Ok((
                Expression::ShiftRight(
                    *loc,
                    Box::new(left),
                    Box::new(cast_shift_arg(loc, right, right_length, &left_type)),
                    left_type.signed(),
                ),
                left_type,
            ))
        }
        ast::Expression::Multiply(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

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
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

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
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

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
            let (base, base_type) = expression(b, cfg, ns, vartab, errors)?;
            let (exp, exp_type) = expression(e, cfg, ns, vartab, errors)?;

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
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

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
                    resolver::Type::bool(),
                ))
            } else {
                Ok((
                    Expression::UMore(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::bool(),
                ))
            }
        }
        ast::Expression::Less(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

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
                    resolver::Type::bool(),
                ))
            } else {
                Ok((
                    Expression::ULess(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::bool(),
                ))
            }
        }
        ast::Expression::MoreEqual(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

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
                    resolver::Type::bool(),
                ))
            } else {
                Ok((
                    Expression::UMoreEqual(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::bool(),
                ))
            }
        }
        ast::Expression::LessEqual(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

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
                    resolver::Type::bool(),
                ))
            } else {
                Ok((
                    Expression::ULessEqual(
                        *loc,
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::bool(),
                ))
            }
        }
        ast::Expression::Equal(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((
                Expression::Equal(
                    *loc,
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                resolver::Type::bool(),
            ))
        }
        ast::Expression::NotEqual(loc, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((
                Expression::NotEqual(
                    *loc,
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                resolver::Type::bool(),
            ))
        }

        // unary expressions
        ast::Expression::Not(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, ns, vartab, errors)?;

            Ok((
                Expression::Not(
                    *loc,
                    Box::new(cast(
                        &loc,
                        expr,
                        &expr_type,
                        &resolver::Type::bool(),
                        true,
                        ns,
                        errors,
                    )?),
                ),
                resolver::Type::bool(),
            ))
        }
        ast::Expression::Complement(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, ns, vartab, errors)?;

            get_int_length(&expr_type, loc, true, ns, errors)?;

            Ok((Expression::Complement(*loc, Box::new(expr)), expr_type))
        }
        ast::Expression::UnaryMinus(loc, e) => {
            let expr: &ast::Expression = e;
            if let ast::Expression::NumberLiteral(loc, n) = expr {
                expression(
                    &ast::Expression::NumberLiteral(*loc, -n),
                    cfg,
                    ns,
                    vartab,
                    errors,
                )
            } else {
                let (expr, expr_type) = expression(e, cfg, ns, vartab, errors)?;

                get_int_length(&expr_type, loc, false, ns, errors)?;

                Ok((Expression::UnaryMinus(*loc, Box::new(expr)), expr_type))
            }
        }
        ast::Expression::UnaryPlus(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, ns, vartab, errors)?;

            get_int_length(&expr_type, loc, false, ns, errors)?;

            Ok((expr, expr_type))
        }

        ast::Expression::Ternary(loc, c, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;
            let (cond, cond_type) = expression(c, cfg, ns, vartab, errors)?;

            let cond = cast(
                &c.loc(),
                cond,
                &cond_type,
                &resolver::Type::bool(),
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

            let v = vartab.find(id, ns, errors)?;

            match v.ty {
                resolver::Type::Primitive(ast::PrimitiveType::Bytes(_))
                | resolver::Type::Primitive(ast::PrimitiveType::Int(_))
                | resolver::Type::Primitive(ast::PrimitiveType::Uint(_)) => (),
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
                                Box::new(Expression::NumberLiteral(*loc, v.ty.bits(), One::one())),
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
                                Box::new(Expression::NumberLiteral(*loc, v.ty.bits(), One::one())),
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
                                Box::new(Expression::NumberLiteral(*loc, v.ty.bits(), One::one())),
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
                                Box::new(Expression::NumberLiteral(*loc, v.ty.bits(), One::one())),
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
            let (expr, expr_type) = expression(e, cfg, ns, vartab, errors)?;

            match var.as_ref() {
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
                    let var = vartab.find(id, ns, errors)?;

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
                    let (var_expr, var_ty) = expression(var, cfg, ns, vartab, errors)?;

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

                    let pos = vartab.temp_anonymous(&var_ty);

                    match var_ty {
                        resolver::Type::Ref(r_ty) => {
                            // reference to memory (e.g. array)
                            cfg.add(
                                vartab,
                                Instr::Set {
                                    res: pos,
                                    expr: cast(
                                        &var.loc(),
                                        expr,
                                        &expr_type,
                                        &r_ty,
                                        true,
                                        ns,
                                        errors,
                                    )?,
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

                            Ok((Expression::Variable(*loc, pos), *r_ty))
                        }
                        resolver::Type::StorageRef(r_ty) => {
                            cfg.add(
                                vartab,
                                Instr::Set {
                                    res: pos,
                                    expr: cast(
                                        &var.loc(),
                                        expr,
                                        &expr_type,
                                        &r_ty,
                                        true,
                                        ns,
                                        errors,
                                    )?,
                                },
                            );

                            // The value of the var_expr should be storage offset
                            cfg.add(
                                vartab,
                                Instr::SetStorage {
                                    ty: *r_ty.clone(),
                                    local: pos,
                                    storage: var_expr,
                                },
                            );

                            cfg.writes_contract_storage = true;

                            Ok((Expression::Variable(*loc, pos), *r_ty))
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
            let (set, set_type) = expression(e, cfg, ns, vartab, errors)?;

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
            match var.as_ref() {
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

                    let v = tab.find(id, ns, errors)?;

                    match v.ty {
                        resolver::Type::Primitive(ast::PrimitiveType::Bytes(_))
                        | resolver::Type::Primitive(ast::PrimitiveType::Int(_))
                        | resolver::Type::Primitive(ast::PrimitiveType::Uint(_)) => (),
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
                    let (var_expr, var_ty) = expression(var, cfg, ns, vartab, errors)?;

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
                        resolver::Type::Ref(r_ty) => match *r_ty {
                            resolver::Type::Primitive(ast::PrimitiveType::Bytes(_))
                            | resolver::Type::Primitive(ast::PrimitiveType::Int(_))
                            | resolver::Type::Primitive(ast::PrimitiveType::Uint(_)) => {
                                let set = op(var_expr.clone(), &*r_ty, errors)?;

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
                                Ok((Expression::Variable(*loc, pos), *r_ty))
                            }
                            _ => {
                                errors.push(Output::error(
                                    var.loc(),
                                    format!("assigning to incorrect type {}", r_ty.to_string(ns)),
                                ));
                                Err(())
                            }
                        },
                        resolver::Type::StorageRef(r_ty) => match *r_ty {
                            resolver::Type::Primitive(ast::PrimitiveType::Bytes(_))
                            | resolver::Type::Primitive(ast::PrimitiveType::Int(_))
                            | resolver::Type::Primitive(ast::PrimitiveType::Uint(_)) => {
                                let set = op(
                                    Expression::StorageLoad(
                                        *loc,
                                        *r_ty.clone(),
                                        Box::new(var_expr.clone()),
                                    ),
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
                                    Instr::SetStorage {
                                        ty: *r_ty.clone(),
                                        storage: var_expr,
                                        local: pos,
                                    },
                                );
                                Ok((Expression::Variable(*loc, pos), *r_ty))
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
        ast::Expression::NamedFunctionCall(loc, ty, args) => {
            let mut blackhole = Vec::new();

            match ns.resolve_type(ty, &mut blackhole) {
                Ok(resolver::Type::Struct(n)) => {
                    return named_struct_literal(loc, n, args, cfg, ns, vartab, errors);
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

            match ty {
                ast::Type::Unresolved(expr) => {
                    let (id, dimensions) = ns.expr_to_type(expr, errors)?;
                    if !dimensions.is_empty() {
                        errors.push(Output::error(*loc, "unexpected array type".to_string()));
                        return Err(());
                    }

                    function_call_with_named_arguments(loc, &id, args, cfg, ns, vartab, errors)
                }
                _ => unreachable!(),
            }
        }
        ast::Expression::New(loc, ty, args) => new(loc, ty, args, cfg, ns, vartab, errors),
        ast::Expression::FunctionCall(loc, ty, args) => {
            let mut blackhole = Vec::new();

            match ns.resolve_type(ty, &mut blackhole) {
                Ok(resolver::Type::Struct(n)) => {
                    return struct_literal(loc, n, args, cfg, ns, vartab, errors);
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
                        let (expr, expr_type) = expression(&args[0], cfg, ns, vartab, errors)?;

                        Ok((cast(loc, expr, &expr_type, &to, false, ns, errors)?, to))
                    };
                }
                Err(_) => {}
            }

            match ty {
                ast::Type::Unresolved(expr) => {
                    if let ast::Expression::MemberAccess(_, member, func) = expr.as_ref() {
                        method_call(loc, member, func, args, cfg, ns, vartab, errors)
                    } else {
                        let (id, dimensions) = ns.expr_to_type(expr, errors)?;

                        if !dimensions.is_empty() {
                            errors.push(Output::error(*loc, "unexpected array type".to_string()));
                            return Err(());
                        }

                        function_call_with_positional_arguments(
                            loc, &id, args, cfg, ns, vartab, errors,
                        )
                    }
                }
                _ => unreachable!(),
            }
        }
        ast::Expression::ArraySubscript(loc, _, None) => {
            errors.push(Output::error(
                *loc,
                "expected expression before ‘]’ token".to_string(),
            ));

            Err(())
        }
        ast::Expression::ArraySubscript(loc, array, Some(index)) => {
            array_subscript(loc, array, index, cfg, ns, vartab, errors)
        }
        ast::Expression::MemberAccess(loc, e, id) => {
            if let ast::Expression::Variable(namespace) = e.as_ref() {
                if let Some(e) = ns.resolve_enum(namespace) {
                    return match ns.enums[e].values.get(&id.name) {
                        Some((_, val)) => Ok((
                            Expression::NumberLiteral(
                                *loc,
                                ns.enums[e].ty.bits(),
                                BigInt::from_usize(*val).unwrap(),
                            ),
                            resolver::Type::Enum(e),
                        )),
                        None => {
                            errors.push(Output::error(
                                id.loc,
                                format!(
                                    "enum {} does not have value {}",
                                    ns.enums[e].name, id.name
                                ),
                            ));
                            Err(())
                        }
                    };
                }
            }

            let (expr, expr_ty) = expression(e, cfg, ns, vartab, errors)?;

            // Dereference if need to. This could be struct-in-struct for
            // example.
            let (expr, expr_ty) = if let resolver::Type::Ref(ty) = expr_ty {
                (Expression::Load(*loc, Box::new(expr)), *ty)
            } else {
                (expr, expr_ty)
            };

            match expr_ty {
                resolver::Type::Primitive(ast::PrimitiveType::Bytes(n)) => {
                    if id.name == "length" {
                        return Ok((
                            Expression::NumberLiteral(*loc, 8, BigInt::from_u8(n).unwrap()),
                            resolver::Type::Primitive(ast::PrimitiveType::Uint(8)),
                        ));
                    }
                }
                resolver::Type::Array(_, dim) => {
                    if id.name == "length" {
                        return match dim.last().unwrap() {
                            None => Ok((
                                Expression::DynamicArrayLength(*loc, Box::new(expr)),
                                resolver::Type::Primitive(ast::PrimitiveType::Uint(32)),
                            )),
                            Some(d) => bigint_to_expression(loc, d, errors),
                        };
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
                    resolver::Type::Primitive(ast::PrimitiveType::Bytes(n)) => {
                        if id.name == "length" {
                            return Ok((
                                Expression::NumberLiteral(*loc, 8, BigInt::from_u8(n).unwrap()),
                                resolver::Type::Primitive(ast::PrimitiveType::Uint(8)),
                            ));
                        }
                    }
                    resolver::Type::Array(_, dim) => {
                        if id.name == "length" {
                            return match dim.last().unwrap() {
                                None => Ok((
                                    expr,
                                    resolver::Type::StorageRef(Box::new(
                                        resolver::Type::Primitive(ast::PrimitiveType::Uint(256)),
                                    )),
                                )),
                                Some(d) => bigint_to_expression(loc, d, errors),
                            };
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
                                ns.structs[n].name, id.name
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
            let boolty = resolver::Type::bool();
            let (l, l_type) = expression(left, cfg, ns, vartab, errors)?;
            let l = cast(&loc, l, &l_type, &boolty, true, ns, errors)?;

            let mut tab = match vartab {
                &mut Some(ref mut tab) => tab,
                None => {
                    // In constant context, no side effects so short-circut not necessary
                    let (r, r_type) = expression(right, cfg, ns, vartab, errors)?;

                    return Ok((
                        Expression::Or(
                            *loc,
                            Box::new(l),
                            Box::new(cast(
                                &loc,
                                r,
                                &r_type,
                                &resolver::Type::bool(),
                                true,
                                ns,
                                errors,
                            )?),
                        ),
                        resolver::Type::bool(),
                    ));
                }
            };

            let pos = tab.temp(
                &ast::Identifier {
                    name: "or".to_owned(),
                    loc: *loc,
                },
                &resolver::Type::bool(),
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

            let (r, r_type) = expression(right, cfg, ns, &mut Some(&mut tab), errors)?;
            let r = cast(&loc, r, &r_type, &resolver::Type::bool(), true, ns, errors)?;

            cfg.add(tab, Instr::Set { res: pos, expr: r });

            let mut phis = HashSet::new();
            phis.insert(pos);

            cfg.set_phis(end_or, phis);

            cfg.add(tab, Instr::Branch { bb: end_or });

            cfg.set_basic_block(end_or);

            Ok((Expression::Variable(*loc, pos), boolty))
        }
        ast::Expression::And(loc, left, right) => {
            let boolty = resolver::Type::bool();
            let (l, l_type) = expression(left, cfg, ns, vartab, errors)?;
            let l = cast(&loc, l, &l_type, &boolty, true, ns, errors)?;

            let mut tab = match vartab {
                &mut Some(ref mut tab) => tab,
                None => {
                    // In constant context, no side effects so short-circut not necessary
                    let (r, r_type) = expression(right, cfg, ns, vartab, errors)?;

                    return Ok((
                        Expression::And(
                            *loc,
                            Box::new(l),
                            Box::new(cast(
                                &loc,
                                r,
                                &r_type,
                                &resolver::Type::bool(),
                                true,
                                ns,
                                errors,
                            )?),
                        ),
                        resolver::Type::bool(),
                    ));
                }
            };

            let pos = tab.temp(
                &ast::Identifier {
                    name: "and".to_owned(),
                    loc: *loc,
                },
                &resolver::Type::bool(),
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

            let (r, r_type) = expression(right, cfg, ns, &mut Some(&mut tab), errors)?;
            let r = cast(&loc, r, &r_type, &resolver::Type::bool(), true, ns, errors)?;

            cfg.add(tab, Instr::Set { res: pos, expr: r });

            let mut phis = HashSet::new();
            phis.insert(pos);

            cfg.set_phis(end_and, phis);

            cfg.add(tab, Instr::Branch { bb: end_and });

            cfg.set_basic_block(end_and);

            Ok((Expression::Variable(*loc, pos), boolty))
        }
        _ => panic!("unimplemented: {:?}", expr),
    }
}

/// Resolve an new expression
fn new(
    loc: &ast::Loc,
    ty: &ast::Type,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    ns: &resolver::Contract,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    // TODO: new can also be used for creating contracts
    let ty = ns.resolve_type(ty, errors)?;

    match &ty {
        resolver::Type::Array(_, dim) => {
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

    let (size_expr, size_ty) = expression(&args[0], cfg, ns, vartab, errors)?;

    let size_width = match size_ty {
        resolver::Type::Primitive(ast::PrimitiveType::Uint(n)) => n,
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
        Ordering::Greater => Expression::Trunc(
            size_loc,
            resolver::Type::Primitive(ast::PrimitiveType::Uint(32)),
            Box::new(size_expr),
        ),
        Ordering::Less => Expression::ZeroExt(
            size_loc,
            resolver::Type::Primitive(ast::PrimitiveType::Uint(32)),
            Box::new(size_expr),
        ),
        Ordering::Equal => size_expr,
    };

    Ok((
        Expression::AllocDynamicArray(*loc, ty.clone(), Box::new(size)),
        ty,
    ))
}

/// Resolve an array subscript expression
fn array_subscript(
    loc: &ast::Loc,
    array: &ast::Expression,
    index: &ast::Expression,
    cfg: &mut ControlFlowGraph,
    ns: &resolver::Contract,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let (mut array_expr, array_ty) = expression(array, cfg, ns, vartab, errors)?;

    let (array_length, array_length_ty) = match array_ty.deref() {
        resolver::Type::Primitive(ast::PrimitiveType::Bytes(n)) => {
            bigint_to_expression(loc, &BigInt::from(*n), errors)?
        }
        resolver::Type::Array(_, _) => match array_ty.array_length() {
            None => {
                if let resolver::Type::StorageRef(_) = array_ty {
                    let array_length = Expression::StorageLoad(
                        *loc,
                        resolver::Type::Primitive(ast::PrimitiveType::Uint(256)),
                        Box::new(array_expr.clone()),
                    );

                    array_expr = Expression::Keccak256(*loc, Box::new(array_expr));

                    (
                        array_length,
                        resolver::Type::Primitive(ast::PrimitiveType::Uint(256)),
                    )
                } else {
                    (
                        Expression::DynamicArrayLength(*loc, Box::new(array_expr.clone())),
                        resolver::Type::Primitive(ast::PrimitiveType::Uint(32)),
                    )
                }
            }
            Some(l) => bigint_to_expression(loc, l, errors)?,
        },
        _ => {
            errors.push(Output::error(
                array.loc(),
                "expression is not an array".to_string(),
            ));
            return Err(());
        }
    };

    let (index_expr, index_ty) = expression(index, cfg, ns, vartab, errors)?;

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
        resolver::Type::Primitive(ast::PrimitiveType::Uint(w)) => w,
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

    let array_width = array_length_ty.bits();
    let width = std::cmp::max(array_width, index_width);
    let coerced_ty = resolver::Type::Primitive(ast::PrimitiveType::Uint(width));

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
    cfg.add(tab, Instr::AssertFailure {});

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
                            resolver::Type::Primitive(ast::PrimitiveType::Uint(256)),
                            Box::new(Expression::Multiply(
                                *loc,
                                Box::new(cast(
                                    &index.loc(),
                                    Expression::Variable(index.loc(), pos),
                                    &coerced_ty,
                                    &resolver::Type::Primitive(ast::PrimitiveType::Uint(64)),
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
                    &resolver::Type::Primitive(ast::PrimitiveType::Uint(256)),
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
            resolver::Type::Primitive(ast::PrimitiveType::Bytes(array_length)) => {
                let res_ty = resolver::Type::Primitive(ast::PrimitiveType::Bytes(1));

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
            resolver::Type::Array(_, _) => Ok((
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
    ns: &resolver::Contract,
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
            let (expr, expr_type) = expression(&a, cfg, ns, vartab, errors)?;

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
fn function_call_with_positional_arguments(
    loc: &ast::Loc,
    id: &ast::Identifier,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    ns: &resolver::Contract,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    // Try to resolve as a function call
    let funcs = ns.resolve_func(&id, errors)?;

    let mut resolved_args = Vec::new();
    let mut resolved_types = Vec::new();

    for arg in args {
        let (expr, expr_type) = expression(arg, cfg, ns, vartab, errors)?;

        resolved_args.push(Box::new(expr));
        resolved_types.push(expr_type);
    }

    let tab = match vartab {
        &mut Some(ref mut tab) => tab,
        None => {
            errors.push(Output::error(
                *loc,
                "cannot call function in constant expression".to_string(),
            ));
            return Err(());
        }
    };

    let mut temp_errors = Vec::new();

    // function call
    for f in funcs {
        let func = &ns.functions[f.1];

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

        if !matches {
            continue;
        }

        // .. what about return value?
        if func.returns.len() > 1 {
            errors.push(Output::error(
                *loc,
                "in expression context a function cannot return more than one value".to_string(),
            ));
            return Err(());
        }

        if !func.returns.is_empty() {
            let ty = &func.returns[0].ty;
            let id = ast::Identifier {
                loc: ast::Loc(0, 0),
                name: "".to_owned(),
            };
            let temp_pos = tab.temp(&id, ty);

            cfg.add(
                tab,
                Instr::Call {
                    res: vec![temp_pos],
                    func: f.1,
                    args: cast_args,
                },
            );

            return Ok((Expression::Variable(id.loc, temp_pos), ty.clone()));
        } else {
            cfg.add(
                tab,
                Instr::Call {
                    res: Vec::new(),
                    func: f.1,
                    args: cast_args,
                },
            );

            return Ok((Expression::Poison, resolver::Type::Undef));
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
fn function_call_with_named_arguments(
    loc: &ast::Loc,
    id: &ast::Identifier,
    args: &[ast::NamedArgument],
    cfg: &mut ControlFlowGraph,
    ns: &resolver::Contract,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    // Try to resolve as a function call
    let funcs = ns.resolve_func(&id, errors)?;

    let mut arguments = HashMap::new();

    for arg in args {
        arguments.insert(
            arg.name.name.to_string(),
            expression(&arg.expr, cfg, ns, vartab, errors)?,
        );
    }

    let tab = match vartab {
        &mut Some(ref mut tab) => tab,
        None => {
            errors.push(Output::error(
                *loc,
                "cannot call function in constant expression".to_string(),
            ));
            return Err(());
        }
    };

    let mut temp_errors = Vec::new();

    // function call
    for f in funcs {
        let func = &ns.functions[f.1];

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

        if !matches {
            continue;
        }

        // .. what about return value?
        if func.returns.len() > 1 {
            errors.push(Output::error(
                *loc,
                "in expression context a function cannot return more than one value".to_string(),
            ));
            return Err(());
        }

        if !func.returns.is_empty() {
            let ty = &func.returns[0].ty;
            let id = ast::Identifier {
                loc: ast::Loc(0, 0),
                name: "".to_owned(),
            };
            let temp_pos = tab.temp(&id, ty);

            cfg.add(
                tab,
                Instr::Call {
                    res: vec![temp_pos],
                    func: f.1,
                    args: cast_args,
                },
            );

            return Ok((Expression::Variable(id.loc, temp_pos), ty.clone()));
        } else {
            cfg.add(
                tab,
                Instr::Call {
                    res: Vec::new(),
                    func: f.1,
                    args: cast_args,
                },
            );

            return Ok((Expression::Poison, resolver::Type::Undef));
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
    ns: &resolver::Contract,
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
                    let (expr, expr_type) = expression(&a.expr, cfg, ns, vartab, errors)?;

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
fn method_call(
    loc: &ast::Loc,
    var: &ast::Expression,
    func: &ast::Identifier,
    args: &[ast::Expression],
    cfg: &mut ControlFlowGraph,
    ns: &resolver::Contract,
    vartab: &mut Option<&mut Vartable>,
    errors: &mut Vec<output::Output>,
) -> Result<(Expression, resolver::Type), ()> {
    let (var_expr, var_ty) = expression(var, cfg, ns, vartab, errors)?;

    if let resolver::Type::StorageRef(ty) = &var_ty {
        if let resolver::Type::Array(_, dim) = ty.as_ref() {
            if func.name == "push" {
                if dim.last().unwrap().is_some() {
                    errors.push(Output::error(
                        func.loc,
                        "method ‘push()’ not allowed on fixed length array".to_string(),
                    ));
                    return Err(());
                }

                let tab = match vartab {
                    &mut Some(ref mut tab) => tab,
                    None => {
                        errors.push(Output::error(
                            *loc,
                            format!("cannot call method ‘{}’ in constant expression", func.name),
                        ));
                        return Err(());
                    }
                };

                if args.len() > 1 {
                    errors.push(Output::error(
                        func.loc,
                        "method ‘push()’ takes at most 1 argument".to_string(),
                    ));
                    return Err(());
                }
                // set array+length to val_expr
                let slot_ty = resolver::Type::Primitive(ast::PrimitiveType::Uint(256));
                let slot_pos = tab.temp_anonymous(&slot_ty);

                cfg.add(
                    tab,
                    Instr::Set {
                        res: slot_pos,
                        expr: Expression::StorageLoad(
                            *loc,
                            slot_ty.clone(),
                            Box::new(var_expr.clone()),
                        ),
                    },
                );

                let elem_ty = ty.array_deref();
                let storage = array_offset(
                    loc,
                    Expression::Keccak256(*loc, Box::new(var_expr.clone())),
                    Expression::Variable(*loc, slot_pos),
                    elem_ty.clone(),
                    ns,
                );

                if args.len() == 1 {
                    let (val_expr, val_ty) = expression(&args[0], cfg, ns, &mut Some(tab), errors)?;

                    let pos = tab.temp_anonymous(&elem_ty);

                    cfg.add(
                        tab,
                        Instr::Set {
                            res: pos,
                            expr: cast(
                                &args[0].loc(),
                                val_expr,
                                &val_ty,
                                &elem_ty.deref(),
                                true,
                                ns,
                                errors,
                            )?,
                        },
                    );

                    cfg.add(
                        tab,
                        Instr::SetStorage {
                            ty: elem_ty,
                            local: pos,
                            storage,
                        },
                    );
                } else {
                    cfg.add(
                        tab,
                        Instr::ClearStorage {
                            ty: elem_ty,
                            storage,
                        },
                    );
                }

                // increase length
                let new_length = tab.temp_anonymous(&slot_ty);

                cfg.add(
                    tab,
                    Instr::Set {
                        res: new_length,
                        expr: Expression::Add(
                            *loc,
                            Box::new(Expression::Variable(*loc, slot_pos)),
                            Box::new(Expression::NumberLiteral(*loc, 256, BigInt::one())),
                        ),
                    },
                );

                cfg.add(
                    tab,
                    Instr::SetStorage {
                        ty: slot_ty,
                        local: new_length,
                        storage: var_expr,
                    },
                );

                return Ok((Expression::Poison, resolver::Type::Undef));
            }
        }
    }

    errors.push(Output::error(
        func.loc,
        format!("method ‘{}’ does not exist", func.name),
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
) -> Expression {
    let to_width = ty.bits();

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
    ns: &resolver::Contract,
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
    let (first, ty) = expression(flattened.next().unwrap(), cfg, ns, vartab, errors)?;

    let mut exprs = vec![first];

    for e in flattened {
        let (mut other, oty) = expression(e, cfg, ns, vartab, errors)?;

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
