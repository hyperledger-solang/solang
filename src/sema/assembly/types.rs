use crate::ast::{Namespace, Type};
use solang_parser::pt::Identifier;
use solang_parser::Diagnostic;

pub(crate) fn get_type_from_string(text: &str) -> Option<Type> {
    match text {
        "bool" => Some(Type::Bool),
        "s8" => Some(Type::Int(8)),
        "s32" => Some(Type::Int(32)),
        "s64" => Some(Type::Int(64)),
        "s128" => Some(Type::Int(128)),
        "s256" => Some(Type::Int(256)),
        "u8" => Some(Type::Uint(8)),
        "u32" => Some(Type::Uint(32)),
        "u64" => Some(Type::Uint(64)),
        "u128" => Some(Type::Uint(128)),
        "u256" => Some(Type::Uint(256)),
        _ => None,
    }
}

pub(crate) fn get_default_type_from_identifier(
    ty: &Option<Identifier>,
    ns: &mut Namespace,
) -> Result<Type, ()> {
    if let Some(type_id) = ty {
        if let Some(asm_type) = get_type_from_string(&type_id.name) {
            Ok(asm_type)
        } else {
            ns.diagnostics.push(Diagnostic::error(
                type_id.loc,
                format!("the specified type '{}' does not exist", type_id.name),
            ));
            Err(())
        }
    } else {
        Ok(Type::Uint(256))
    }
}
