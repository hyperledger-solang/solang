use crate::ast::{Namespace, Symbol, Type};
use crate::sema::assembly::builtin::{
    assembly_unsupported_builtin, parse_builtin_keyword, AssemblyBuiltInFunction,
};
use crate::sema::assembly::functions::{AssemblyFunction, AssemblyFunctionParameter};
use crate::sema::assembly::types::{get_default_type_from_identifier, get_type_from_string};
use crate::sema::expression::{unescape, ExprContext};
use crate::sema::symtable::{Symtable, VariableUsage};
use indexmap::IndexMap;
use num_bigint::{BigInt, Sign};
use num_traits::Num;
use solang_parser::diagnostics::{ErrorType, Level};
use solang_parser::pt::{AssemblyFunctionCall, CodeLocation, Identifier, Loc, StorageLocation};
use solang_parser::{pt, Diagnostic};

// TODO: State variables cannot be assigned to, only .slot can be assigned to, .length cannot be assigned to (check TypeChecker.cpp)
// TODO: Add assembly to unused variable detection

#[derive(PartialEq, Debug)]
pub enum AssemblyExpression {
    BoolLiteral(pt::Loc, bool, Type),
    NumberLiteral(pt::Loc, BigInt, Type),
    StringLiteral(pt::Loc, Vec<u8>, Type),
    AssemblyLocalVariable(pt::Loc, Type, usize),
    SolidityLocalVariable(pt::Loc, Type, Option<StorageLocation>, usize),
    ConstantVariable(pt::Loc, Type, Option<usize>, usize),
    StorageVariable(pt::Loc, Type, usize, usize),
    BuiltInCall(pt::Loc, AssemblyBuiltInFunction, Vec<AssemblyExpression>),
    FunctionCall(pt::Loc, String, Vec<AssemblyExpression>),
    MemberAccess(pt::Loc, Box<AssemblyExpression>, AssemblySuffix),
}

#[derive(PartialEq, Debug)]
pub enum AssemblySuffix {
    Offset,
    Slot,
    Length,
    Selector,
    Address,
}

fn get_suffix_from_string(suffix_name: &str) -> Option<AssemblySuffix> {
    match suffix_name {
        "offset" => Some(AssemblySuffix::Offset),
        "slot" => Some(AssemblySuffix::Slot),
        "length" => Some(AssemblySuffix::Length),
        "selector" => Some(AssemblySuffix::Selector),
        "address" => Some(AssemblySuffix::Address),
        _ => None,
    }
}

impl CodeLocation for AssemblyExpression {
    fn loc(&self) -> pt::Loc {
        match self {
            AssemblyExpression::BoolLiteral(loc, ..)
            | AssemblyExpression::NumberLiteral(loc, ..)
            | AssemblyExpression::StringLiteral(loc, ..)
            | AssemblyExpression::AssemblyLocalVariable(loc, ..)
            | AssemblyExpression::SolidityLocalVariable(loc, ..)
            | AssemblyExpression::ConstantVariable(loc, ..)
            | AssemblyExpression::StorageVariable(loc, ..)
            | AssemblyExpression::BuiltInCall(loc, ..)
            | AssemblyExpression::MemberAccess(loc, ..)
            | AssemblyExpression::FunctionCall(loc, ..) => *loc,
        }
    }
}

// TODO: remove this decorator. It avoids warnings during development
#[allow(dead_code)]
pub(crate) fn resolve_assembly_expression(
    expr: &pt::AssemblyExpression,
    context: &ExprContext,
    symtable: &Symtable,
    functions: &IndexMap<String, AssemblyFunction>,
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

        pt::AssemblyExpression::Variable(id) => {
            resolve_variable_reference(id, ns, symtable, context)
        }

        pt::AssemblyExpression::FunctionCall(func_call) => {
            resolve_function_call(functions, func_call, context, symtable, ns)
        }

        pt::AssemblyExpression::Member(loc, expr, id) => {
            resolve_member_access(loc, expr, id, context, symtable, functions, ns)
        }
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

fn resolve_variable_reference(
    id: &pt::Identifier,
    ns: &mut Namespace,
    symtable: &Symtable,
    context: &ExprContext,
) -> Result<AssemblyExpression, ()> {
    if let Some(v) = symtable.find(&id.name) {
        match &v.usage_type {
            VariableUsage::AssemblyLocalVariable => {
                return Ok(AssemblyExpression::AssemblyLocalVariable(
                    id.loc,
                    v.ty.clone(),
                    v.pos,
                ))
            }
            VariableUsage::AnonymousReturnVariable => {
                unreachable!("Anonymous returns variables cannot be accessed from assembly blocks")
            }
            _ => {
                return Ok(AssemblyExpression::SolidityLocalVariable(
                    id.loc,
                    v.ty.clone(),
                    v.storage_location.clone(),
                    v.pos,
                ))
            }
        }
    }

    match ns.resolve_var(context.file_no, context.contract_no, id, false) {
        Some(Symbol::Variable(_, Some(var_contract_no), var_no)) => {
            let var = &ns.contracts[*var_contract_no].variables[*var_no];
            if var.constant {
                Ok(AssemblyExpression::ConstantVariable(
                    id.loc,
                    var.ty.clone(),
                    Some(*var_contract_no),
                    *var_no,
                ))
            } else {
                Ok(AssemblyExpression::StorageVariable(
                    id.loc,
                    var.ty.clone(),
                    *var_contract_no,
                    *var_no,
                ))
            }
        }
        Some(Symbol::Variable(_, None, var_no)) => {
            let var = &ns.constants[*var_no];
            Ok(AssemblyExpression::ConstantVariable(
                id.loc,
                var.ty.clone(),
                None,
                *var_no,
            ))
        }
        None => {
            ns.diagnostics.push(Diagnostic::decl_error(
                id.loc,
                format!("'{}' is not found", id.name),
            ));
            Err(())
        }

        _ => {
            ns.diagnostics.push(Diagnostic::error(
                id.loc,
                "only variables can be accessed inside assembly blocks".to_string(),
            ));
            Err(())
        }
    }
}

fn resolve_function_call(
    functions: &IndexMap<String, AssemblyFunction>,
    func_call: &AssemblyFunctionCall,
    context: &ExprContext,
    symtable: &Symtable,
    ns: &mut Namespace,
) -> Result<AssemblyExpression, ()> {
    if func_call.id.name.starts_with("verbatim") {
        ns.diagnostics.push(Diagnostic::error(
            func_call.id.loc,
            "verbatim functions are not yet supported in Solang".to_string(),
        ));
        return Err(());
    } else if assembly_unsupported_builtin(func_call.id.name.as_str()) {
        ns.diagnostics.push(Diagnostic::error(
            func_call.id.loc,
            format!(
                "the internal EVM built-in '{}' is not yet supported",
                func_call.id.name
            ),
        ));
        return Err(());
    }

    let mut resolved_arguments: Vec<AssemblyExpression> =
        Vec::with_capacity(func_call.arguments.len());
    for item in &func_call.arguments {
        let resolved_expr = resolve_assembly_expression(item, context, symtable, functions, ns)?;

        if let Some(diagnostic) = check_type(&resolved_expr) {
            ns.diagnostics.push(diagnostic);
            return Err(());
        }

        resolved_arguments.push(resolved_expr);
    }

    if let Some(built_in) = parse_builtin_keyword(&func_call.id.name[..]) {
        let prototype = &built_in.get_prototype_info();
        if prototype.no_args as usize != func_call.arguments.len() {
            ns.diagnostics.push(Diagnostic {
                level: Level::Error,
                ty: ErrorType::TypeError,
                pos: func_call.loc,
                message: format!(
                    "builtin function '{}' requires {} arguments, but {} were provided",
                    prototype.name,
                    prototype.no_args,
                    func_call.arguments.len()
                ),
                notes: vec![],
            });
            return Err(());
        }

        let default_builtin_parameter = AssemblyFunctionParameter {
            loc: Loc::Builtin,
            name: Identifier {
                loc: Loc::Builtin,
                name: "".to_string(),
            },
            ty: Type::Uint(256),
        };

        for item in &resolved_arguments {
            check_function_argument(&default_builtin_parameter, item, functions, ns);
        }

        return Ok(AssemblyExpression::BuiltInCall(
            func_call.loc,
            *built_in,
            resolved_arguments,
        ));
    }

    if let Some(func) = functions.get(&func_call.id.name) {
        if resolved_arguments.len() != func.params.len() {
            ns.diagnostics.push(Diagnostic::error(
                func_call.loc,
                format!(
                    "function '{}' requires {} arguments, but {} were provided",
                    func_call.id.name,
                    func.params.len(),
                    resolved_arguments.len()
                ),
            ));
            return Err(());
        }

        for (index, item) in func.params.iter().enumerate() {
            check_function_argument(item, &resolved_arguments[index], functions, ns);
        }

        return Ok(AssemblyExpression::FunctionCall(
            func_call.id.loc,
            func_call.id.name.clone(),
            resolved_arguments,
        ));
    }

    ns.diagnostics.push(Diagnostic::error(
        func_call.id.loc,
        format!("function '{}' is not defined", func_call.id.name),
    ));

    Err(())
}

fn check_function_argument(
    parameter: &AssemblyFunctionParameter,
    argument: &AssemblyExpression,
    functions: &IndexMap<String, AssemblyFunction>,
    ns: &mut Namespace,
) {
    let arg_type = match argument {
        AssemblyExpression::BoolLiteral(..) => Type::Bool,

        AssemblyExpression::NumberLiteral(_, _, ty)
        | AssemblyExpression::StringLiteral(_, _, ty)
        | AssemblyExpression::AssemblyLocalVariable(_, ty, _)
        | AssemblyExpression::ConstantVariable(_, ty, ..)
        | AssemblyExpression::SolidityLocalVariable(_, ty, None, _) => ty.clone(),

        AssemblyExpression::SolidityLocalVariable(_, _, Some(_), _)
        | AssemblyExpression::MemberAccess(..)
        | AssemblyExpression::StorageVariable(..) => Type::Uint(256),

        AssemblyExpression::BuiltInCall(_, ty, _) => {
            let prototype = ty.get_prototype_info();
            if prototype.no_returns == 0 {
                ns.diagnostics.push(Diagnostic::error(
                    argument.loc(),
                    format!("builtin function '{}' returns nothing", prototype.name),
                ));
                Type::Void
            } else if prototype.no_args > 1 {
                ns.diagnostics.push(Diagnostic::error(
                    argument.loc(),
                    format!(
                        "builtin function '{}' has multiple returns and cannot be used as argument",
                        prototype.name
                    ),
                ));
                Type::Unreachable
            } else {
                Type::Uint(256)
            }
        }

        AssemblyExpression::FunctionCall(_, name, ..) => {
            let func = functions.get(name).unwrap();
            if func.returns.is_empty() {
                ns.diagnostics.push(Diagnostic::error(
                    argument.loc(),
                    format!("function '{}' returns nothing", func.name),
                ));
                Type::Void
            } else if func.returns.len() > 1 {
                ns.diagnostics.push(Diagnostic::error(
                    argument.loc(),
                    format!(
                        "function '{}' has multiple returns and cannot be used as argument",
                        func.name
                    ),
                ));
                Type::Unreachable
            } else {
                func.returns[0].ty.clone()
            }
        }
    };

    if matches!(parameter.ty, Type::Bool) && !matches!(arg_type, Type::Bool) {
        ns.diagnostics.push(Diagnostic::warning(
            argument.loc(),
            "Truncating argument to bool".to_string(),
        ));
    } else if (matches!(parameter.ty, Type::Uint(_)) && matches!(arg_type, Type::Uint(_)))
        || (matches!(parameter.ty, Type::Int(_)) && matches!(arg_type, Type::Int(_)))
    {
        let n1 = parameter.ty.get_type_size();
        let n2 = arg_type.get_type_size();
        if n1 > n2 {
            ns.diagnostics.push(Diagnostic::warning(
                argument.loc(),
                format!("{}-bit type may not fit into '{}'-bit type", n1, n2),
            ));
        }
    } else if matches!(parameter.ty, Type::Uint(_)) && matches!(arg_type, Type::Int(_)) {
        ns.diagnostics.push(Diagnostic::warning(
            argument.loc(),
            "Singed integer may not be correctly represented as unsigned integer".to_string(),
        ));
    } else if matches!(parameter.ty, Type::Int(_)) && matches!(arg_type, Type::Uint(_)) {
        let n1 = parameter.ty.get_type_size();
        let n2 = arg_type.get_type_size();
        if n1 == n2 {
            ns.diagnostics.push(Diagnostic::warning(
                argument.loc(),
                format!(
                    "{}-bit unsigned integer may not fit into {}-bit signed integer",
                    n1, n2
                ),
            ));
        }
    }
}

fn resolve_member_access(
    loc: &pt::Loc,
    expr: &pt::AssemblyExpression,
    id: &Identifier,
    context: &ExprContext,
    symtable: &Symtable,
    functions: &IndexMap<String, AssemblyFunction>,
    ns: &mut Namespace,
) -> Result<AssemblyExpression, ()> {
    let suffix_type = match get_suffix_from_string(&id.name[..]) {
        Some(suffix) => suffix,
        None => {
            ns.diagnostics.push(Diagnostic::error(
                id.loc,
                "the provided suffix is not allowed in yul".to_string(),
            ));
            return Err(());
        }
    };

    let resolved_expr = resolve_assembly_expression(expr, context, symtable, functions, ns)?;
    match resolved_expr {
        AssemblyExpression::ConstantVariable(_, _, Some(_), _) => {
            ns.diagnostics.push(Diagnostic::error(
                resolved_expr.loc(),
                "the suffixes .offset and .slot can only be used in non-constant storage variables"
                    .to_string(),
            ));
            return Err(());
        }

        AssemblyExpression::SolidityLocalVariable(
            _,
            Type::Array(_, ref dims),
            Some(StorageLocation::Calldata(_)),
            _,
        ) => {
            if dims[0].is_none() && id.name != "offset" && id.name != "length" {
                ns.diagnostics.push(Diagnostic::error(
                    resolved_expr.loc(),
                    "calldata variables only support \".offset\" and \".length\"".to_string(),
                ));
            }
            return Err(());
        }

        AssemblyExpression::SolidityLocalVariable(_, Type::InternalFunction { .. }, ..)
        | AssemblyExpression::ConstantVariable(_, Type::InternalFunction { .. }, ..)
        | AssemblyExpression::StorageVariable(_, Type::InternalFunction { .. }, ..) => {
            ns.diagnostics.push(Diagnostic::error(
                resolved_expr.loc(),
                "only variables of type external function pointer support suffixes".to_string(),
            ));
            return Err(());
        }

        AssemblyExpression::SolidityLocalVariable(_, Type::ExternalFunction { .. }, ..)
        | AssemblyExpression::ConstantVariable(_, Type::ExternalFunction { .. }, ..)
        | AssemblyExpression::StorageVariable(_, Type::ExternalFunction { .. }, ..) => {
            if id.name != "selector" && id.name != "address" {
                ns.diagnostics.push(Diagnostic::error(
                    id.loc,
                    "variables of type function pointer only support \".selector\" and \".address\" suffixes".to_string()
                ));
                return Err(());
            }
        }

        AssemblyExpression::SolidityLocalVariable(_, _, Some(StorageLocation::Storage(_)), _)
        | AssemblyExpression::StorageVariable(_, _, _, _) => {
            if id.name != "slot" && id.name != "offset" {
                ns.diagnostics.push(Diagnostic::error(
                    id.loc,
                    "state variables only support \".slot\" and \".offset\"".to_string(),
                ));
                return Err(());
            }
        }

        AssemblyExpression::MemberAccess(..) => {
            ns.diagnostics.push(Diagnostic::error(
                id.loc,
                "there cannot be multiple suffixes to a name".to_string(),
            ));
            return Err(());
        }

        AssemblyExpression::BoolLiteral(..)
        | AssemblyExpression::NumberLiteral(..)
        | AssemblyExpression::StringLiteral(..)
        | AssemblyExpression::AssemblyLocalVariable(..)
        | AssemblyExpression::BuiltInCall(..)
        | AssemblyExpression::FunctionCall(..)
        | AssemblyExpression::ConstantVariable(_, _, None, _) => {
            ns.diagnostics.push(Diagnostic::error(
                resolved_expr.loc(),
                "the given expression does not support suffixes".to_string(),
            ));
            return Err(());
        }

        _ => (),
    }

    Ok(AssemblyExpression::MemberAccess(
        *loc,
        Box::new(resolved_expr),
        suffix_type,
    ))
}

pub(crate) fn check_type(expr: &AssemblyExpression) -> Option<Diagnostic> {
    match expr {
        AssemblyExpression::SolidityLocalVariable(_, _, Some(StorageLocation::Storage(_)), ..)
        | AssemblyExpression::StorageVariable(..) => {
            return Some(Diagnostic::error(
                expr.loc(),
                "Storage variables must be accessed with \".slot\" or \".offset\"".to_string(),
            ));
        }

        AssemblyExpression::SolidityLocalVariable(
            _,
            Type::Array(_, ref dims),
            Some(StorageLocation::Calldata(_)),
            ..,
        ) => {
            if dims[0].is_none() {
                return Some(Diagnostic::error(
                    expr.loc(),
                    "Calldata arrays must be accessed with \".offset\", \".length\" and the \"calldatacopy\" function".to_string()
                ));
            }
        }

        _ => (),
    }

    None
}
