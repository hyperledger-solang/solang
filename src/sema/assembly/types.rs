use crate::ast::{Namespace, Type};
use crate::pt::CodeLocation;
use crate::sema::assembly::expression::AssemblyExpression;
use crate::sema::assembly::functions::FunctionsTable;
use solang_parser::pt::Identifier;
use solang_parser::Diagnostic;

/// Retrieves an YUL type from a keyword
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

/// Given a parse tree identifier, retrieve its type or return the default for YUL
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

/// Performs checks on whether it is possible to retrieve a type from an expression
pub(crate) fn verify_type_from_expression(
    expr: &AssemblyExpression,
    function_table: &FunctionsTable,
) -> Result<Type, Diagnostic> {
    match expr {
        AssemblyExpression::BoolLiteral(..) => Ok(Type::Bool),

        AssemblyExpression::NumberLiteral(_, _, ty)
        | AssemblyExpression::StringLiteral(_, _, ty)
        | AssemblyExpression::AssemblyLocalVariable(_, ty, _)
        | AssemblyExpression::ConstantVariable(_, ty, ..)
        | AssemblyExpression::SolidityLocalVariable(_, ty, None, _) => Ok(ty.clone()),

        AssemblyExpression::SolidityLocalVariable(_, _, Some(_), _)
        | AssemblyExpression::MemberAccess(..)
        | AssemblyExpression::StorageVariable(..) => Ok(Type::Uint(256)),

        AssemblyExpression::BuiltInCall(_, ty, _) => {
            let prototype = ty.get_prototype_info();
            if prototype.no_returns == 0 {
                Err(Diagnostic::error(
                    expr.loc(),
                    format!("builtin function '{}' returns nothing", prototype.name),
                ))
            } else if prototype.no_returns > 1 {
                Err(Diagnostic::error(
                    expr.loc(),
                    format!(
                        "builtin function '{}' has multiple returns and cannot be used in this scope",
                        prototype.name
                    ),
                ))
            } else {
                Ok(Type::Uint(256))
            }
        }

        AssemblyExpression::FunctionCall(_, function_no, ..) => {
            let func = function_table.get(*function_no).unwrap();
            if func.returns.is_empty() {
                Err(Diagnostic::error(
                    expr.loc(),
                    format!("function '{}' returns nothing", func.id.name),
                ))
            } else if func.returns.len() > 1 {
                Err(Diagnostic::error(
                    expr.loc(),
                    format!(
                        "function '{}' has multiple returns and cannot be used in this scope",
                        func.id.name
                    ),
                ))
            } else {
                Ok(func.returns[0].ty.clone())
            }
        }
    }
}
