use crate::ast::{Namespace, Type};
use crate::sema::assembly::types::{get_default_type_from_identifier, get_type_from_string};
use crate::sema::expression::unescape;
use num_bigint::{BigInt, Sign};
use num_traits::Num;
use solang_parser::diagnostics::{ErrorType, Level};
use solang_parser::{pt, Diagnostic};

#[derive(PartialEq, Debug)]
pub enum AssemblyExpression {
    BoolLiteral(pt::Loc, bool, Type),
    NumberLiteral(pt::Loc, BigInt, Type),
    StringLiteral(pt::Loc, Vec<u8>, Type),
    Variable(pt::Loc, usize),
}

// Avoid warnings during development
#[allow(dead_code)]
pub(crate) fn resolve_assembly_expression(
    expr: &pt::AssemblyExpression,
    ns: &mut Namespace,
) -> Result<AssemblyExpression, ()> {
    match expr {
        pt::AssemblyExpression::BoolLiteral(loc, value, ty) => {
            resolve_bool_literal(loc, value, ty, ns)
        }

        pt::AssemblyExpression::NumberLiteral(loc, value, ty) => {
            resolve_number_literal(loc, value, ty, ns)
        }

        pt::AssemblyExpression::HexNumberLiteral(loc, value, ty) => {
            resolve_hex_literal(loc, value, ty, ns)
        }
        pt::AssemblyExpression::HexStringLiteral(value, ty) => {
            if (value.hex.len() % 2) != 0 {
                ns.diagnostics.push(Diagnostic {
                    pos: value.loc,
                    ty: ErrorType::DeclarationError,
                    level: Level::Error,
                    message: format!("hex string \"{}\" has odd number of characters", value.hex),
                    notes: vec![],
                });
                return Err(());
            }

            let mut byte_array: Vec<u8> = Vec::new();
            byte_array.extend_from_slice(&hex::decode(&value.hex).unwrap());

            resolve_string_literal(&value.loc, byte_array, ty, ns)
        }

        pt::AssemblyExpression::StringLiteral(value, ty) => {
            let unescaped_string = unescape(
                &value.string[..],
                0,
                value.loc.file_no(),
                &mut ns.diagnostics,
            );
            resolve_string_literal(&value.loc, unescaped_string, ty, ns)
        }

        // TODO: This is a workaround to avoid compilation errors, while we don't finish resolving all expressions
        _ => Ok(AssemblyExpression::Variable(pt::Loc::File(0, 0, 0), 0)),
    }
}

fn get_type_from_big_int(big_int: &BigInt) -> Type {
    match big_int.sign() {
        Sign::Minus => Type::Int(256),
        _ => Type::Uint(256),
    }
}

fn resolve_bool_literal(
    loc: &pt::Loc,
    value: &bool,
    ty: &Option<pt::Identifier>,
    ns: &mut Namespace,
) -> Result<AssemblyExpression, ()> {
    let new_type = if let Some(type_id) = ty {
        if let Some(asm_type) = get_type_from_string(&type_id.name) {
            asm_type
        } else {
            ns.diagnostics.push(Diagnostic::error(
                type_id.loc,
                format!("the specified type '{}' does not exist", type_id.name),
            ));
            return Err(());
        }
    } else {
        Type::Bool
    };

    Ok(AssemblyExpression::BoolLiteral(*loc, *value, new_type))
}

fn resolve_number_literal(
    loc: &pt::Loc,
    value: &BigInt,
    ty: &Option<pt::Identifier>,
    ns: &mut Namespace,
) -> Result<AssemblyExpression, ()> {
    let new_type = if let Some(type_id) = ty {
        if let Some(asm_type) = get_type_from_string(&type_id.name) {
            if matches!(asm_type, Type::Uint(_)) && matches!(value.sign(), Sign::Minus) {
                ns.diagnostics.push(Diagnostic {
                    pos: *loc,
                    level: Level::Error,
                    ty: ErrorType::TypeError,
                    message: "singed value cannot fit in unsigned type".to_string(),
                    notes: vec![],
                });
                return Err(());
            }
            asm_type
        } else {
            ns.diagnostics.push(Diagnostic::error(
                type_id.loc,
                format!("the specified type '{}' does not exist", type_id.name),
            ));
            return Err(());
        }
    } else {
        get_type_from_big_int(value)
    };

    let type_size = new_type.get_type_size();

    let bits_needed = match value.sign() {
        Sign::Minus => value.bits() + 1,
        _ => value.bits(),
    };

    if bits_needed > type_size as u64 {
        ns.diagnostics.push(Diagnostic {
            level: Level::Error,
            ty: ErrorType::TypeError,
            pos: *loc,
            message: format!(
                "the provided literal requires {} bits, but the type only supports {}",
                bits_needed, type_size
            ),
            notes: vec![],
        });
    }

    Ok(AssemblyExpression::NumberLiteral(
        *loc,
        value.clone(),
        new_type,
    ))
}

fn resolve_hex_literal(
    loc: &pt::Loc,
    value: &str,
    ty: &Option<pt::Identifier>,
    ns: &mut Namespace,
) -> Result<AssemblyExpression, ()> {
    let new_type = get_default_type_from_identifier(ty, ns)?;

    let s: String = value.chars().skip(2).filter(|v| *v != '_').collect();
    let val = BigInt::from_str_radix(&s, 16).unwrap();
    let type_size = new_type.get_type_size();
    if val.bits() > type_size as u64 {
        ns.diagnostics.push(Diagnostic {
            level: Level::Error,
            ty: ErrorType::TypeError,
            pos: *loc,
            message: format!(
                "the provided literal requires {} bits, but the type only supports {}",
                val.bits(),
                type_size
            ),
            notes: vec![],
        });
    }

    Ok(AssemblyExpression::NumberLiteral(*loc, val, new_type))
}

fn resolve_string_literal(
    loc: &pt::Loc,
    byte_array: Vec<u8>,
    ty: &Option<pt::Identifier>,
    ns: &mut Namespace,
) -> Result<AssemblyExpression, ()> {
    let new_type = get_default_type_from_identifier(ty, ns)?;
    let type_size = new_type.get_type_size();

    if byte_array.len() * 8 > type_size as usize {
        ns.diagnostics.push(Diagnostic {
            level: Level::Error,
            ty: ErrorType::DeclarationError,
            pos: *loc,
            message: format!(
                "the provided literal requires {} bits, but the type only supports {}",
                byte_array.len() * 8,
                type_size
            ),
            notes: vec![],
        });
    }

    Ok(AssemblyExpression::StringLiteral(
        *loc, byte_array, new_type,
    ))
}
