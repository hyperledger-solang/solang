use crate::ast::{Namespace, Symbol, Type};
use crate::sema::assembly::ast::{AssemblyExpression, AssemblyFunctionParameter, AssemblySuffix};
use crate::sema::assembly::builtin::{assembly_unsupported_builtin, parse_builtin_keyword};
use crate::sema::assembly::functions::FunctionsTable;
use crate::sema::assembly::types::{
    get_default_type_from_identifier, get_type_from_string, verify_type_from_expression,
};
use crate::sema::assembly::unused_variable::{assigned_variable, used_variable};
use crate::sema::expression::{unescape, ExprContext};
use crate::sema::symtable::{Symtable, VariableUsage};
use num_bigint::{BigInt, Sign};
use num_traits::Num;
use solang_parser::diagnostics::{ErrorType, Level};
use solang_parser::pt::{YulFunctionCall, CodeLocation, Identifier, Loc, StorageLocation};
use solang_parser::{pt, Diagnostic};

/// Given a keyword, returns the suffix it represents in YUL
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

/// Resolve an assembly expression.
pub(crate) fn resolve_assembly_expression(
    expr: &pt::YulExpression,
    context: &ExprContext,
    symtable: &mut Symtable,
    function_table: &mut FunctionsTable,
    ns: &mut Namespace,
) -> Result<AssemblyExpression, ()> {
    match expr {
        pt::YulExpression::BoolLiteral(loc, value, ty) => {
            resolve_bool_literal(loc, value, ty, ns)
        }

        pt::YulExpression::NumberLiteral(loc, value, ty) => {
            resolve_number_literal(loc, value, ty, ns)
        }

        pt::YulExpression::HexNumberLiteral(loc, value, ty) => {
            resolve_hex_literal(loc, value, ty, ns)
        }
        pt::YulExpression::HexStringLiteral(value, ty) => {
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

        pt::YulExpression::StringLiteral(value, ty) => {
            let unescaped_string = unescape(
                &value.string[..],
                0,
                value.loc.file_no(),
                &mut ns.diagnostics,
            );
            resolve_string_literal(&value.loc, unescaped_string, ty, ns)
        }

        pt::YulExpression::Variable(id) => {
            resolve_variable_reference(id, ns, symtable, context)
        }

        pt::YulExpression::FunctionCall(func_call) => {
            resolve_function_call(function_table, func_call, context, symtable, ns)
        }

        pt::YulExpression::Member(loc, expr, id) => {
            resolve_member_access(loc, expr, id, context, symtable, function_table, ns)
        }
    }
}

/// Returns the default YUL type a bigint represents
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
                    message: "signed integer cannot fit in unsigned integer".to_string(),
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

    // yul functions cannot access contract symbols
    if !context.yul_function {
        return match ns.resolve_var(context.file_no, context.contract_no, id, false) {
            Some(Symbol::Variable(_, Some(var_contract_no), var_no)) => {
                let var = &ns.contracts[*var_contract_no].variables[*var_no];
                if var.immutable {
                    ns.diagnostics.push(Diagnostic::error(
                        id.loc,
                        "assembly access to immutable variables is not supported".to_string(),
                    ));
                    return Err(());
                }

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
                ns.diagnostics.push(Diagnostic::error(
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
        };
    }

    ns.diagnostics.push(Diagnostic::error(
        id.loc,
        format!("'{}' is not found", id.name),
    ));
    Err(())
}

pub(crate) fn resolve_function_call(
    function_table: &mut FunctionsTable,
    func_call: &YulFunctionCall,
    context: &ExprContext,
    symtable: &mut Symtable,
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
        let resolved_expr =
            resolve_assembly_expression(item, context, symtable, function_table, ns)?;

        if let Some(diagnostic) = check_type(&resolved_expr, context, ns, symtable) {
            ns.diagnostics.push(diagnostic);
            return Err(());
        }

        resolved_arguments.push(resolved_expr);
    }

    if let Some(built_in) = parse_builtin_keyword(func_call.id.name.as_str()) {
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
            id: Identifier {
                loc: Loc::Builtin,
                name: "".to_string(),
            },
            ty: Type::Uint(256),
        };

        for item in &resolved_arguments {
            check_function_argument(&default_builtin_parameter, item, function_table, ns);
        }

        return Ok(AssemblyExpression::BuiltInCall(
            func_call.loc,
            *built_in,
            resolved_arguments,
        ));
    }

    if let Some(func) = function_table.find(&func_call.id.name) {
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
            check_function_argument(item, &resolved_arguments[index], function_table, ns);
        }

        let fn_no = func.function_no;
        let resolved_fn = Ok(AssemblyExpression::FunctionCall(
            func_call.id.loc,
            fn_no,
            resolved_arguments,
        ));
        function_table.function_called(fn_no);
        return resolved_fn;
    }

    ns.diagnostics.push(Diagnostic::error(
        func_call.id.loc,
        format!("function '{}' is not defined", func_call.id.name),
    ));

    Err(())
}

/// Check if the provided argument is compatible with the declared parameters of a function.
fn check_function_argument(
    parameter: &AssemblyFunctionParameter,
    argument: &AssemblyExpression,
    function_table: &FunctionsTable,
    ns: &mut Namespace,
) {
    let arg_type = match verify_type_from_expression(argument, function_table) {
        Ok(ty) => ty,
        Err(diagnostic) => {
            ns.diagnostics.push(diagnostic);
            Type::Unreachable
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
        if n1 < n2 {
            ns.diagnostics.push(Diagnostic::warning(
                argument.loc(),
                format!("{} bit type may not fit into {} bit type", n2, n1),
            ));
        }
    } else if matches!(parameter.ty, Type::Uint(_)) && matches!(arg_type, Type::Int(_)) {
        ns.diagnostics.push(Diagnostic::warning(
            argument.loc(),
            "signed integer may not be correctly represented as unsigned integer".to_string(),
        ));
    } else if matches!(parameter.ty, Type::Int(_)) && matches!(arg_type, Type::Uint(_)) {
        let n1 = parameter.ty.get_type_size();
        let n2 = arg_type.get_type_size();
        if n1 == n2 {
            ns.diagnostics.push(Diagnostic::warning(
                argument.loc(),
                format!(
                    "{} bit unsigned integer may not fit into {} bit signed integer",
                    n1, n2
                ),
            ));
        }
    }
}

/// Resolve variables accessed with suffixes (e.g. 'var.slot', 'var.offset')
fn resolve_member_access(
    loc: &pt::Loc,
    expr: &pt::YulExpression,
    id: &Identifier,
    context: &ExprContext,
    symtable: &mut Symtable,
    function_table: &mut FunctionsTable,
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

    let resolved_expr = resolve_assembly_expression(expr, context, symtable, function_table, ns)?;
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
                return Err(());
            }
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

/// Check if an assembly expression has been used correctly in a assignment or if the member access
/// has a valid expression given the context.
pub(crate) fn check_type(
    expr: &AssemblyExpression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
) -> Option<Diagnostic> {
    if context.lvalue {
        match expr {
            AssemblyExpression::SolidityLocalVariable(
                _,
                _,
                Some(StorageLocation::Storage(_)),
                ..,
            )
            | AssemblyExpression::StorageVariable(..) => {
                return Some(Diagnostic::error(
                    expr.loc(),
                    "storage variables cannot be assigned any value in assembly. You may use ‘sstore()‘".to_string()
                ));
            }

            AssemblyExpression::StringLiteral(..)
            | AssemblyExpression::NumberLiteral(..)
            | AssemblyExpression::BoolLiteral(..)
            | AssemblyExpression::ConstantVariable(..) => {
                return Some(Diagnostic::error(
                    expr.loc(),
                    "cannot assigned a value to a constant".to_string(),
                ));
            }

            AssemblyExpression::BuiltInCall(..) | AssemblyExpression::FunctionCall(..) => {
                return Some(Diagnostic::error(
                    expr.loc(),
                    "cannot assign a value to a function".to_string(),
                ));
            }

            AssemblyExpression::MemberAccess(_, _, AssemblySuffix::Length) => {
                return Some(Diagnostic::error(
                    expr.loc(),
                    "cannot assign a value to length".to_string(),
                ));
            }

            AssemblyExpression::MemberAccess(_, _, AssemblySuffix::Offset) => {
                return Some(Diagnostic::error(
                    expr.loc(),
                    "cannot assign a value to offset".to_string(),
                ));
            }
            AssemblyExpression::MemberAccess(_, exp, AssemblySuffix::Slot) => {
                if matches!(**exp, AssemblyExpression::StorageVariable(..)) {
                    return Some(Diagnostic::error(
                        exp.loc(),
                        "cannot assign to slot of storage variable".to_string(),
                    ));
                }
            }

            _ => (),
        }

        assigned_variable(ns, expr, symtable);
    } else {
        used_variable(ns, expr, symtable);
    }

    match expr {
        AssemblyExpression::SolidityLocalVariable(_, _, Some(StorageLocation::Storage(_)), ..)
        | AssemblyExpression::StorageVariable(..) => {
            return Some(Diagnostic::error(
                expr.loc(),
                "Storage variables must be accessed with ‘.slot‘ or ‘.offset‘".to_string(),
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
                    "Calldata arrays must be accessed with ‘.offset‘, ‘.length‘ and the ‘calldatacopy‘ function".to_string()
                ));
            }
        }

        _ => (),
    }

    None
}
