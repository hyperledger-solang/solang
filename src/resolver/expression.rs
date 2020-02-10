use num_bigint::BigInt;
use num_bigint::Sign;
use num_traits::FromPrimitive;
use num_traits::Num;
use num_traits::One;
use num_traits::ToPrimitive;
use num_traits::Zero;
use std::cmp;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::ops::Mul;
use unescape::unescape;

use hex;
use output;
use output::Output;
use parser::ast;
use resolver;
use resolver::address::to_hexstr_eip55;
use resolver::cfg::{ControlFlowGraph, Instr, Storage, Vartable};

#[derive(PartialEq, Clone, Debug)]
pub enum Expression {
    BoolLiteral(bool),
    BytesLiteral(Vec<u8>),
    NumberLiteral(u16, BigInt),
    StructLiteral(resolver::Type, Vec<Expression>),
    ArrayLiteral(resolver::Type, Vec<u32>, Vec<Expression>),
    ConstArrayLiteral(Vec<u32>, Vec<Expression>),
    Add(Box<Expression>, Box<Expression>),
    Subtract(Box<Expression>, Box<Expression>),
    Multiply(Box<Expression>, Box<Expression>),
    UDivide(Box<Expression>, Box<Expression>),
    SDivide(Box<Expression>, Box<Expression>),
    UModulo(Box<Expression>, Box<Expression>),
    SModulo(Box<Expression>, Box<Expression>),
    Power(Box<Expression>, Box<Expression>),
    BitwiseOr(Box<Expression>, Box<Expression>),
    BitwiseAnd(Box<Expression>, Box<Expression>),
    BitwiseXor(Box<Expression>, Box<Expression>),
    ShiftLeft(Box<Expression>, Box<Expression>),
    ShiftRight(Box<Expression>, Box<Expression>, bool),
    Variable(ast::Loc, usize),
    Load(Box<Expression>),
    StorageLoad(resolver::Type, Box<Expression>),
    ZeroExt(resolver::Type, Box<Expression>),
    SignExt(resolver::Type, Box<Expression>),
    Trunc(resolver::Type, Box<Expression>),

    UMore(Box<Expression>, Box<Expression>),
    ULess(Box<Expression>, Box<Expression>),
    UMoreEqual(Box<Expression>, Box<Expression>),
    ULessEqual(Box<Expression>, Box<Expression>),
    SMore(Box<Expression>, Box<Expression>),
    SLess(Box<Expression>, Box<Expression>),
    SMoreEqual(Box<Expression>, Box<Expression>),
    SLessEqual(Box<Expression>, Box<Expression>),
    Equal(Box<Expression>, Box<Expression>),
    NotEqual(Box<Expression>, Box<Expression>),

    Not(Box<Expression>),
    Complement(Box<Expression>),
    UnaryMinus(Box<Expression>),

    Ternary(Box<Expression>, Box<Expression>, Box<Expression>),
    ArraySubscript(Box<Expression>, Box<Expression>),
    StructMember(Box<Expression>, usize),

    Or(Box<Expression>, Box<Expression>),
    And(Box<Expression>, Box<Expression>),

    Poison,
}

impl Expression {
    /// Returns true if the Expression may load from contract storage using StorageLoad
    pub fn reads_contract_storage(&self) -> bool {
        match self {
            Expression::StorageLoad(_, _) => true,
            Expression::BoolLiteral(_)
            | Expression::BytesLiteral(_)
            | Expression::NumberLiteral(_, _) => false,
            Expression::StructLiteral(_, exprs) => exprs.iter().any(|e| e.reads_contract_storage()),
            Expression::ArrayLiteral(_, _, exprs) => {
                exprs.iter().any(|e| e.reads_contract_storage())
            }
            Expression::ConstArrayLiteral(_, _) => false,
            Expression::Add(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::Subtract(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::Multiply(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::UDivide(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::SDivide(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::UModulo(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::SModulo(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),

            Expression::Power(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::BitwiseOr(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::BitwiseAnd(l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::BitwiseXor(l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::ShiftLeft(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::ShiftRight(l, r, _) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }

            Expression::Variable(_, _) | Expression::Load(_) => false,
            Expression::ZeroExt(_, e) => e.reads_contract_storage(),
            Expression::SignExt(_, e) => e.reads_contract_storage(),
            Expression::Trunc(_, e) => e.reads_contract_storage(),

            Expression::UMore(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::ULess(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::UMoreEqual(l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::ULessEqual(l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::SMore(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::SLess(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::SLessEqual(l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::SMoreEqual(l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::Equal(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::NotEqual(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),

            Expression::Not(e) => e.reads_contract_storage(),
            Expression::Complement(e) => e.reads_contract_storage(),
            Expression::UnaryMinus(e) => e.reads_contract_storage(),

            Expression::Ternary(c, l, r) => {
                c.reads_contract_storage()
                    || l.reads_contract_storage()
                    || r.reads_contract_storage()
            }
            Expression::ArraySubscript(l, r) => {
                l.reads_contract_storage() || r.reads_contract_storage()
            }
            Expression::StructMember(s, _) => s.reads_contract_storage(),
            Expression::And(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::Or(l, r) => l.reads_contract_storage() || r.reads_contract_storage(),
            Expression::Poison => false,
        }
    }

    /// Print the expression to string. This assumes the expression is a compile-time constant
    /// without any references to e.g. variables.
    pub fn print_constant_to_string(&self, ns: &resolver::Contract) -> String {
        match self {
            Expression::NumberLiteral(_, n) => n.to_string(),
            Expression::Add(l, r) => format!(
                "({} + {})",
                l.print_constant_to_string(ns),
                r.print_constant_to_string(ns)
            ),
            Expression::Subtract(l, r) => format!(
                "({} - {})",
                l.print_constant_to_string(ns),
                r.print_constant_to_string(ns)
            ),
            Expression::Multiply(l, r) => format!(
                "({} * {})",
                l.print_constant_to_string(ns),
                r.print_constant_to_string(ns)
            ),
            // FIXME: more to be implemented. Not needed for storage references
            _ => unimplemented!(),
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
        resolver::Type::FixedArray(_, _) => {
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
                Expression::NumberLiteral(int_size, n.clone()),
                resolver::Type::Primitive(ast::PrimitiveType::Int(int_size)),
            ))
        }
    } else if bits > 256 {
        errors.push(Output::error(*loc, format!("{} is too large", n)));
        Err(())
    } else {
        Ok((
            Expression::NumberLiteral(int_size, n.clone()),
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
            Expression::Load(Box::new(expr)),
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
            Expression::StorageLoad(*r.clone(), Box::new(expr)),
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
            &Expression::NumberLiteral(_, ref n),
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
                Ok(Expression::NumberLiteral(to_len, n.clone()))
            }
        }
        (
            &Expression::NumberLiteral(_, ref n),
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
                Ok(Expression::NumberLiteral(to_len, n.clone()))
            }
        }
        // Literal strings can be implicitly lengthened
        (
            &Expression::BytesLiteral(ref bs),
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

                Ok(Expression::BytesLiteral(bs))
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
                    Ok(Expression::Trunc(to.clone(), Box::new(expr)))
                }
            }
            Ordering::Less => Ok(Expression::ZeroExt(to.clone(), Box::new(expr))),
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
                    Ok(Expression::Trunc(to.clone(), Box::new(expr)))
                }
            }
            Ordering::Less => Ok(Expression::SignExt(to.clone(), Box::new(expr))),
            Ordering::Equal => Ok(expr),
        },
        (
            resolver::Type::Primitive(ast::PrimitiveType::Uint(from_len)),
            resolver::Type::Primitive(ast::PrimitiveType::Int(to_len)),
        ) if to_len > from_len => Ok(Expression::ZeroExt(to.clone(), Box::new(expr))),
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
                Ok(Expression::Trunc(to.clone(), Box::new(expr)))
            } else if from_len < to_len {
                Ok(Expression::SignExt(to.clone(), Box::new(expr)))
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
                Ok(Expression::Trunc(to.clone(), Box::new(expr)))
            } else if from_len < to_len {
                Ok(Expression::ZeroExt(to.clone(), Box::new(expr)))
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
                Ok(Expression::Trunc(to.clone(), Box::new(expr)))
            } else if from_len < 160 {
                Ok(Expression::ZeroExt(to.clone(), Box::new(expr)))
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
                Ok(Expression::Trunc(to.clone(), Box::new(expr)))
            } else if to_len > 160 {
                Ok(Expression::ZeroExt(to.clone(), Box::new(expr)))
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
                    Box::new(Expression::ZeroExt(to.clone(), Box::new(expr))),
                    Box::new(Expression::NumberLiteral(
                        to_len as u16 * 8,
                        BigInt::from_u8(shift).unwrap(),
                    )),
                ))
            } else {
                let shift = (from_len - to_len) * 8;

                Ok(Expression::Trunc(
                    to.clone(),
                    Box::new(Expression::ShiftRight(
                        Box::new(expr),
                        Box::new(Expression::NumberLiteral(
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
            if let Expression::BytesLiteral(from_str) = &expr {
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
        ast::Expression::BoolLiteral(_, v) => Ok((
            Expression::BoolLiteral(*v),
            resolver::Type::Primitive(ast::PrimitiveType::Bool),
        )),
        ast::Expression::StringLiteral(v) => {
            // Concatenate the strings
            let mut result = String::new();

            for s in v {
                // unescape supports octal escape values, solc does not
                // neither solc nor unescape support unicode code points like \u{61}
                match unescape(&s.string) {
                    Some(v) => result.push_str(&v),
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
                Expression::BytesLiteral(result.into_bytes()),
                resolver::Type::Primitive(ast::PrimitiveType::Bytes(length as u8)),
            ))
        }
        ast::Expression::HexLiteral(v) => {
            let mut result = Vec::new();

            for s in v {
                if (s.hex.len() % 2) != 0 {
                    errors.push(Output::error(
                        s.loc,
                        format!("hex string \"{}\" has odd number of characters", s.hex),
                    ));
                    return Err(());
                } else {
                    result.extend_from_slice(&hex::decode(&s.hex).unwrap());
                }
            }

            let length = result.len();

            Ok((
                Expression::BytesLiteral(result),
                resolver::Type::Primitive(ast::PrimitiveType::Bytes(length as u8)),
            ))
        }
        ast::Expression::NumberLiteral(loc, b) => bigint_to_expression(loc, b, errors),
        ast::Expression::AddressLiteral(loc, n) => {
            let address = to_hexstr_eip55(n);

            if address == *n {
                let s: String = address.chars().skip(2).collect();

                Ok((
                    Expression::NumberLiteral(160, BigInt::from_str_radix(&s, 16).unwrap()),
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
                        Expression::NumberLiteral(256, n.clone()),
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
        ast::Expression::Add(_, l, r) => {
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
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                ty,
            ))
        }
        ast::Expression::Subtract(_, l, r) => {
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
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                ty,
            ))
        }
        ast::Expression::BitwiseOr(_, l, r) => {
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
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                ty,
            ))
        }
        ast::Expression::BitwiseAnd(_, l, r) => {
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
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                ty,
            ))
        }
        ast::Expression::BitwiseXor(_, l, r) => {
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
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                ty,
            ))
        }
        ast::Expression::ShiftLeft(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            // left hand side may be bytes/int/uint
            // right hand size may be int/uint
            let _ = get_int_length(&left_type, &l.loc(), true, ns, errors)?;
            let (right_length, _) = get_int_length(&right_type, &r.loc(), false, ns, errors)?;

            Ok((
                Expression::ShiftLeft(
                    Box::new(left),
                    Box::new(cast_shift_arg(right, right_length, &left_type)),
                ),
                left_type,
            ))
        }
        ast::Expression::ShiftRight(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            // left hand side may be bytes/int/uint
            // right hand size may be int/uint
            let _ = get_int_length(&left_type, &l.loc(), true, ns, errors)?;
            let (right_length, _) = get_int_length(&right_type, &r.loc(), false, ns, errors)?;

            Ok((
                Expression::ShiftRight(
                    Box::new(left),
                    Box::new(cast_shift_arg(right, right_length, &left_type)),
                    left_type.signed(),
                ),
                left_type,
            ))
        }
        ast::Expression::Multiply(_, l, r) => {
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
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                ty,
            ))
        }
        ast::Expression::Divide(_, l, r) => {
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
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    ty,
                ))
            } else {
                Ok((
                    Expression::UDivide(
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    ty,
                ))
            }
        }
        ast::Expression::Modulo(_, l, r) => {
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
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    ty,
                ))
            } else {
                Ok((
                    Expression::UModulo(
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
                    Box::new(cast(&b.loc(), base, &base_type, &ty, true, ns, errors)?),
                    Box::new(cast(&e.loc(), exp, &exp_type, &ty, true, ns, errors)?),
                ),
                ty,
            ))
        }

        // compare
        ast::Expression::More(_, l, r) => {
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
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::bool(),
                ))
            } else {
                Ok((
                    Expression::UMore(
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::bool(),
                ))
            }
        }
        ast::Expression::Less(_, l, r) => {
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
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::bool(),
                ))
            } else {
                Ok((
                    Expression::ULess(
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::bool(),
                ))
            }
        }
        ast::Expression::MoreEqual(_, l, r) => {
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
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::bool(),
                ))
            } else {
                Ok((
                    Expression::UMoreEqual(
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::bool(),
                ))
            }
        }
        ast::Expression::LessEqual(_, l, r) => {
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
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::bool(),
                ))
            } else {
                Ok((
                    Expression::ULessEqual(
                        Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                        Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                    ),
                    resolver::Type::bool(),
                ))
            }
        }
        ast::Expression::Equal(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((
                Expression::Equal(
                    Box::new(cast(&l.loc(), left, &left_type, &ty, true, ns, errors)?),
                    Box::new(cast(&r.loc(), right, &right_type, &ty, true, ns, errors)?),
                ),
                resolver::Type::bool(),
            ))
        }
        ast::Expression::NotEqual(_, l, r) => {
            let (left, left_type) = expression(l, cfg, ns, vartab, errors)?;
            let (right, right_type) = expression(r, cfg, ns, vartab, errors)?;

            let ty = coerce(&left_type, &l.loc(), &right_type, &r.loc(), ns, errors)?;

            Ok((
                Expression::NotEqual(
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
                Expression::Not(Box::new(cast(
                    &loc,
                    expr,
                    &expr_type,
                    &resolver::Type::bool(),
                    true,
                    ns,
                    errors,
                )?)),
                resolver::Type::bool(),
            ))
        }
        ast::Expression::Complement(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, ns, vartab, errors)?;

            get_int_length(&expr_type, loc, true, ns, errors)?;

            Ok((Expression::Complement(Box::new(expr)), expr_type))
        }
        ast::Expression::UnaryMinus(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, ns, vartab, errors)?;

            get_int_length(&expr_type, loc, false, ns, errors)?;

            Ok((Expression::UnaryMinus(Box::new(expr)), expr_type))
        }
        ast::Expression::UnaryPlus(loc, e) => {
            let (expr, expr_type) = expression(e, cfg, ns, vartab, errors)?;

            get_int_length(&expr_type, loc, false, ns, errors)?;

            Ok((expr, expr_type))
        }

        ast::Expression::Ternary(_, c, l, r) => {
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
                Expression::Ternary(Box::new(cond), Box::new(left), Box::new(right)),
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
                    v.ty.clone(),
                    Box::new(Expression::NumberLiteral(256, n.clone())),
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
                                Box::new(Expression::Variable(id.loc, v.pos)),
                                Box::new(Expression::NumberLiteral(v.ty.bits(), One::one())),
                            ),
                        },
                    );

                    if let Storage::Contract(n) = &v.storage {
                        cfg.writes_contract_storage = true;
                        cfg.add(
                            vartab,
                            Instr::SetStorage {
                                local: v.pos,
                                storage: Expression::NumberLiteral(256, n.clone()),
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
                                Box::new(Expression::Variable(id.loc, temp_pos)),
                                Box::new(Expression::NumberLiteral(v.ty.bits(), One::one())),
                            ),
                        },
                    );

                    if let Storage::Contract(n) = &v.storage {
                        cfg.writes_contract_storage = true;
                        cfg.add(
                            vartab,
                            Instr::SetStorage {
                                local: v.pos,
                                storage: Expression::NumberLiteral(256, n.clone()),
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
                                Box::new(lvalue),
                                Box::new(Expression::NumberLiteral(v.ty.bits(), One::one())),
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
                                local: v.pos,
                                storage: Expression::NumberLiteral(256, n.clone()),
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
                                Box::new(lvalue),
                                Box::new(Expression::NumberLiteral(v.ty.bits(), One::one())),
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
                                local: v.pos,
                                storage: Expression::NumberLiteral(256, n.clone()),
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
                                    local: var.pos,
                                    storage: Expression::NumberLiteral(256, n.clone()),
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
                            Expression::SignExt(ty.clone(), Box::new(set))
                        } else if right_length < left_length && !set_type.signed() {
                            Expression::ZeroExt(ty.clone(), Box::new(set))
                        } else {
                            Expression::Trunc(ty.clone(), Box::new(set))
                        }
                    }
                    _ => cast(&var.loc(), set, &set_type, &ty, true, ns, errors)?,
                };

                Ok(match expr {
                    ast::Expression::AssignAdd(_, _, _) => {
                        Expression::Add(Box::new(assign), Box::new(set))
                    }
                    ast::Expression::AssignSubtract(_, _, _) => {
                        Expression::Subtract(Box::new(assign), Box::new(set))
                    }
                    ast::Expression::AssignMultiply(_, _, _) => {
                        Expression::Multiply(Box::new(assign), Box::new(set))
                    }
                    ast::Expression::AssignOr(_, _, _) => {
                        Expression::BitwiseOr(Box::new(assign), Box::new(set))
                    }
                    ast::Expression::AssignAnd(_, _, _) => {
                        Expression::BitwiseAnd(Box::new(assign), Box::new(set))
                    }
                    ast::Expression::AssignXor(_, _, _) => {
                        Expression::BitwiseXor(Box::new(assign), Box::new(set))
                    }
                    ast::Expression::AssignShiftLeft(_, _, _) => {
                        Expression::ShiftLeft(Box::new(assign), Box::new(set))
                    }
                    ast::Expression::AssignShiftRight(_, _, _) => {
                        Expression::ShiftRight(Box::new(assign), Box::new(set), ty.signed())
                    }
                    ast::Expression::AssignDivide(_, _, _) => {
                        if ty.signed() {
                            Expression::SDivide(Box::new(assign), Box::new(set))
                        } else {
                            Expression::UDivide(Box::new(assign), Box::new(set))
                        }
                    }
                    ast::Expression::AssignModulo(_, _, _) => {
                        if ty.signed() {
                            Expression::SModulo(Box::new(assign), Box::new(set))
                        } else {
                            Expression::UModulo(Box::new(assign), Box::new(set))
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
                            v.ty.clone(),
                            Box::new(Expression::NumberLiteral(256, n.clone())),
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
                                    local: v.pos,
                                    storage: Expression::NumberLiteral(256, n.clone()),
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
            match ns.resolve_type(ty, None) {
                Ok(resolver::Type::Struct(n)) => {
                    let struct_def = &ns.structs[n];

                    return if args.len() != struct_def.fields.len() {
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
                                        expression(&a.expr, cfg, ns, vartab, errors)?;

                                    fields[i] =
                                        cast(loc, expr, &expr_type, &f.ty, true, ns, errors)?;
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

                        let ty = resolver::Type::Struct(n);

                        Ok((Expression::StructLiteral(ty.clone(), fields), ty))
                    };
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

            // FIXME: function call
            unimplemented!();
        }
        ast::Expression::FunctionCall(loc, ty, args) => {
            match ns.resolve_type(ty, None) {
                Ok(resolver::Type::Struct(n)) => {
                    let struct_def = &ns.structs[n];

                    return if args.len() != struct_def.fields.len() {
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

                        let ty = resolver::Type::Struct(n);

                        Ok((Expression::StructLiteral(ty.clone(), fields), ty))
                    };
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

            let funcs = if let ast::Type::Unresolved(s, _) = ty {
                ns.resolve_func(s, errors)?
            } else {
                unreachable!();
            };

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
                        "in expression context a function cannot return more than one value"
                            .to_string(),
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
        ast::Expression::ArraySubscript(loc, _, None) => {
            errors.push(Output::error(
                *loc,
                "expected expression before ‘]’ token".to_string(),
            ));

            Err(())
        }
        ast::Expression::ArraySubscript(loc, array, Some(index)) => {
            let (array_expr, array_ty) = expression(array, cfg, ns, vartab, errors)?;

            let array_length = match if let resolver::Type::StorageRef(ty) = &array_ty {
                &*ty
            } else {
                &array_ty
            } {
                resolver::Type::Primitive(ast::PrimitiveType::Bytes(n)) => BigInt::from(*n),
                resolver::Type::FixedArray(_, _) => array_ty.array_length().clone(),
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

            let (index_width, _) = get_int_length(&index_ty, &index.loc(), false, ns, errors)?;
            let array_width = array_length.bits();

            let pos = tab.temp(
                &ast::Identifier {
                    name: "index".to_owned(),
                    loc: *loc,
                },
                &index_ty,
            );

            cfg.add(
                tab,
                Instr::Set {
                    res: pos,
                    expr: index_expr,
                },
            );

            let out_of_bounds = cfg.new_basic_block("out_of_bounds".to_string());
            let in_bounds = cfg.new_basic_block("in_bounds".to_string());

            if index_ty.signed() {
                // first check that our index is not negative
                let positive = cfg.new_basic_block("positive".to_string());

                cfg.add(
                    tab,
                    Instr::BranchCond {
                        cond: Expression::SLess(
                            Box::new(Expression::Variable(index.loc(), pos)),
                            Box::new(Expression::NumberLiteral(index_width, BigInt::zero())),
                        ),
                        true_: out_of_bounds,
                        false_: positive,
                    },
                );

                cfg.set_basic_block(positive);

                // If the index if of less bits than the array length, don't bother checking
                if index_width as usize >= array_width {
                    cfg.add(
                        tab,
                        Instr::BranchCond {
                            cond: Expression::SMoreEqual(
                                Box::new(Expression::Variable(index.loc(), pos)),
                                Box::new(Expression::NumberLiteral(
                                    index_width,
                                    array_length.clone(),
                                )),
                            ),
                            true_: out_of_bounds,
                            false_: in_bounds,
                        },
                    );
                } else {
                    cfg.add(tab, Instr::Branch { bb: in_bounds });
                }
            } else if index_width as usize >= array_width {
                cfg.add(
                    tab,
                    Instr::BranchCond {
                        cond: Expression::UMoreEqual(
                            Box::new(Expression::Variable(index.loc(), pos)),
                            Box::new(Expression::NumberLiteral(index_width, array_length.clone())),
                        ),
                        true_: out_of_bounds,
                        false_: in_bounds,
                    },
                );
            } else {
                // if the index is less bits than the array, it is always in bounds
                cfg.add(tab, Instr::Branch { bb: in_bounds });
            }

            cfg.set_basic_block(out_of_bounds);
            cfg.add(tab, Instr::AssertFailure {});

            cfg.set_basic_block(in_bounds);

            match array_ty {
                resolver::Type::StorageRef(ty) => {
                    let elem_ty = ty.storage_deref();
                    let elem_size = elem_ty.storage_slots(ns);
                    if array_length.mul(elem_size).to_u64().is_some() {
                        // we need to calculate the storage offset. If this can be done with 64 bit
                        // arithmetic it will be much more efficient on wasm
                        Ok((
                            Expression::Add(
                                Box::new(array_expr),
                                Box::new(Expression::ZeroExt(
                                    resolver::Type::Primitive(ast::PrimitiveType::Uint(256)),
                                    Box::new(Expression::Multiply(
                                        Box::new(cast(
                                            &index.loc(),
                                            Expression::Variable(index.loc(), pos),
                                            &index_ty,
                                            &resolver::Type::Primitive(ast::PrimitiveType::Uint(
                                                64,
                                            )),
                                            false,
                                            ns,
                                            errors,
                                        )?),
                                        Box::new(Expression::NumberLiteral(
                                            64,
                                            elem_ty.storage_slots(ns),
                                        )),
                                    )),
                                )),
                            ),
                            elem_ty,
                        ))
                    } else {
                        // the index needs to be cast to i256 and multiplied by the number
                        // of slots for each element
                        // FIXME: if elem_size is power-of-2 then shift.
                        Ok((
                            Expression::Add(
                                Box::new(array_expr),
                                Box::new(Expression::Multiply(
                                    Box::new(cast(
                                        &index.loc(),
                                        Expression::Variable(index.loc(), pos),
                                        &index_ty,
                                        &resolver::Type::Primitive(ast::PrimitiveType::Uint(256)),
                                        false,
                                        ns,
                                        errors,
                                    )?),
                                    Box::new(Expression::NumberLiteral(
                                        256,
                                        elem_ty.storage_slots(ns),
                                    )),
                                )),
                            ),
                            elem_ty,
                        ))
                    }
                }
                resolver::Type::Primitive(ast::PrimitiveType::Bytes(array_length)) => {
                    let res_ty = resolver::Type::Primitive(ast::PrimitiveType::Bytes(1));

                    Ok((
                        Expression::Trunc(
                            res_ty.clone(),
                            Box::new(Expression::ShiftRight(
                                Box::new(array_expr),
                                // shift by (array_length - 1 - index) * 8
                                Box::new(Expression::ShiftLeft(
                                    Box::new(Expression::Subtract(
                                        Box::new(Expression::NumberLiteral(
                                            array_length as u16 * 8,
                                            BigInt::from_u8(array_length - 1).unwrap(),
                                        )),
                                        Box::new(cast_shift_arg(
                                            Expression::Variable(index.loc(), pos),
                                            index_width,
                                            &array_ty,
                                        )),
                                    )),
                                    Box::new(Expression::NumberLiteral(
                                        array_length as u16 * 8,
                                        BigInt::from_u8(3).unwrap(),
                                    )),
                                )),
                                false,
                            )),
                        ),
                        res_ty,
                    ))
                }
                resolver::Type::FixedArray(_, _) => Ok((
                    Expression::ArraySubscript(
                        Box::new(array_expr),
                        Box::new(Expression::Variable(index.loc(), pos)),
                    ),
                    array_ty.deref(),
                )),
                _ => {
                    // should not happen as type-checking already done
                    unreachable!();
                }
            }
        }
        ast::Expression::MemberAccess(loc, e, id) => {
            if let ast::Expression::Variable(namespace) = e.as_ref() {
                if let Some(e) = ns.resolve_enum(namespace) {
                    return match ns.enums[e].values.get(&id.name) {
                        Some((_, val)) => Ok((
                            Expression::NumberLiteral(
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
                (Expression::Load(Box::new(expr)), *ty)
            } else {
                (expr, expr_ty)
            };

            match expr_ty {
                resolver::Type::Primitive(ast::PrimitiveType::Bytes(n)) => {
                    if id.name == "length" {
                        return Ok((
                            Expression::NumberLiteral(8, BigInt::from_u8(n).unwrap()),
                            resolver::Type::Primitive(ast::PrimitiveType::Uint(8)),
                        ));
                    }
                }
                resolver::Type::FixedArray(_, dim) => {
                    if id.name == "length" {
                        return bigint_to_expression(loc, dim.last().unwrap(), errors);
                    }
                }
                resolver::Type::StorageRef(r) => {
                    if let resolver::Type::Struct(n) = *r {
                        let mut slot = BigInt::zero();

                        for field in &ns.structs[n].fields {
                            if id.name == field.name {
                                return Ok((
                                    Expression::Add(
                                        Box::new(expr),
                                        Box::new(Expression::NumberLiteral(256, slot)),
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
                }
                resolver::Type::Struct(n) => {
                    if let Some((i, f)) = ns.structs[n]
                        .fields
                        .iter()
                        .enumerate()
                        .find(|f| id.name == f.1.name)
                    {
                        return Ok((
                            Expression::StructMember(Box::new(expr), i),
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
                    expr: Expression::BoolLiteral(true),
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
                    expr: Expression::BoolLiteral(false),
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

// When generating shifts, llvm wants both arguments to have the same width. We want the
// result of the shift to be left argument, so this function coercies the right argument
// into the right length.
fn cast_shift_arg(expr: Expression, from_width: u16, ty: &resolver::Type) -> Expression {
    let to_width = ty.bits();

    if from_width == to_width {
        expr
    } else if from_width < to_width && ty.signed() {
        Expression::SignExt(ty.clone(), Box::new(expr))
    } else if from_width < to_width && !ty.signed() {
        Expression::ZeroExt(ty.clone(), Box::new(expr))
    } else {
        Expression::Trunc(ty.clone(), Box::new(expr))
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

    let aty = resolver::Type::FixedArray(
        Box::new(ty),
        dims.iter()
            .map(|n| BigInt::from_u32(*n).unwrap())
            .collect::<Vec<BigInt>>(),
    );

    if vartab.is_none() {
        Ok((Expression::ConstArrayLiteral(*dims, exprs), aty))
    } else {
        Ok((Expression::ArrayLiteral(aty.clone(), *dims, exprs), aty))
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
