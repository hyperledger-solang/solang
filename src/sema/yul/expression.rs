// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{ArrayLength, Namespace, Parameter, Symbol, Type};
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::{unescape, ExprContext};
use crate::sema::symtable::{Symtable, VariableUsage};
use crate::sema::yul::ast::{YulExpression, YulSuffix};
use crate::sema::yul::builtin::{parse_builtin_keyword, yul_unsupported_builtin};
use crate::sema::yul::functions::FunctionsTable;
use crate::sema::yul::types::{
    get_default_type_from_identifier, get_type_from_string, verify_type_from_expression,
};
use crate::sema::yul::unused_variable::{assigned_variable, used_variable};
use num_bigint::{BigInt, Sign};
use num_rational::BigRational;
use num_traits::{Num, Pow};
use solang_parser::diagnostics::{ErrorType, Level};
use solang_parser::pt::{CodeLocation, Identifier, Loc, StorageLocation, YulFunctionCall};
use solang_parser::{diagnostics::Diagnostic, pt};
use std::{ops::Mul, str::FromStr};

/// Given a keyword, returns the suffix it represents in YUL
fn get_suffix_from_string(suffix_name: &str) -> Option<YulSuffix> {
    match suffix_name {
        "offset" => Some(YulSuffix::Offset),
        "slot" => Some(YulSuffix::Slot),
        "length" => Some(YulSuffix::Length),
        "selector" => Some(YulSuffix::Selector),
        "address" => Some(YulSuffix::Address),
        _ => None,
    }
}

/// Resolve an yul expression.
pub(crate) fn resolve_yul_expression(
    expr: &pt::YulExpression,
    context: &ExprContext,
    symtable: &mut Symtable,
    function_table: &mut FunctionsTable,
    ns: &mut Namespace,
) -> Result<YulExpression, ()> {
    match expr {
        pt::YulExpression::BoolLiteral(loc, value, ty) => resolve_bool_literal(loc, value, ty, ns),

        pt::YulExpression::NumberLiteral(loc, base, exp, ty) => {
            resolve_number_literal(loc, base, exp, ty, ns)
        }

        pt::YulExpression::HexNumberLiteral(loc, value, ty) => {
            resolve_hex_literal(loc, value, ty, ns)
        }
        pt::YulExpression::HexStringLiteral(value, ty) => {
            if (value.hex.len() % 2) != 0 {
                ns.diagnostics.push(Diagnostic {
                    loc: value.loc,
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
            let mut diagnostics = Diagnostics::default();
            let unescaped_string =
                unescape(&value.string[..], 0, value.loc.file_no(), &mut diagnostics);
            ns.diagnostics.extend(diagnostics);
            resolve_string_literal(&value.loc, unescaped_string, ty, ns)
        }

        pt::YulExpression::Variable(id) => resolve_variable_reference(id, ns, symtable, context),

        pt::YulExpression::FunctionCall(func_call) => {
            resolve_function_call(function_table, func_call, context, symtable, ns)
        }

        pt::YulExpression::SuffixAccess(loc, expr, id) => {
            resolve_suffix_access(loc, expr, id, context, symtable, function_table, ns)
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
) -> Result<YulExpression, ()> {
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

    Ok(YulExpression::BoolLiteral(*loc, *value, new_type))
}

fn resolve_number_literal(
    loc: &pt::Loc,
    integer: &str,
    exp: &str,
    ty: &Option<pt::Identifier>,
    ns: &mut Namespace,
) -> Result<YulExpression, ()> {
    let integer = BigInt::from_str(integer).unwrap();

    let value = if exp.is_empty() {
        integer
    } else {
        let base10 = BigInt::from_str("10").unwrap();

        if let Some(abs_exp) = exp.strip_prefix('-') {
            if let Ok(exp) = u8::from_str(abs_exp) {
                let res = BigRational::new(integer, base10.pow(exp));

                if res.is_integer() {
                    res.to_integer()
                } else {
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        "rational numbers not permitted".to_string(),
                    ));
                    return Err(());
                }
            } else {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    format!("exponent '{}' too large", exp),
                ));
                return Err(());
            }
        } else if let Ok(exp) = u8::from_str(exp) {
            integer.mul(base10.pow(exp))
        } else {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                format!("exponent '{}' too large", exp),
            ));
            return Err(());
        }
    };

    let new_type = if let Some(type_id) = ty {
        if let Some(asm_type) = get_type_from_string(&type_id.name) {
            if matches!(asm_type, Type::Uint(_)) && matches!(value.sign(), Sign::Minus) {
                ns.diagnostics.push(Diagnostic {
                    loc: *loc,
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
        get_type_from_big_int(&value)
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
            loc: *loc,
            message: format!(
                "the provided literal requires {} bits, but the type only supports {}",
                bits_needed, type_size
            ),
            notes: vec![],
        });
    }

    Ok(YulExpression::NumberLiteral(*loc, value, new_type))
}

fn resolve_hex_literal(
    loc: &pt::Loc,
    value: &str,
    ty: &Option<pt::Identifier>,
    ns: &mut Namespace,
) -> Result<YulExpression, ()> {
    let new_type = get_default_type_from_identifier(ty, ns)?;

    let s: String = value.chars().skip(2).filter(|v| *v != '_').collect();
    let val = BigInt::from_str_radix(&s, 16).unwrap();
    let type_size = new_type.get_type_size();
    if val.bits() > type_size as u64 {
        ns.diagnostics.push(Diagnostic {
            level: Level::Error,
            ty: ErrorType::TypeError,
            loc: *loc,
            message: format!(
                "the provided literal requires {} bits, but the type only supports {}",
                val.bits(),
                type_size
            ),
            notes: vec![],
        });
    }

    Ok(YulExpression::NumberLiteral(*loc, val, new_type))
}

fn resolve_string_literal(
    loc: &pt::Loc,
    byte_array: Vec<u8>,
    ty: &Option<pt::Identifier>,
    ns: &mut Namespace,
) -> Result<YulExpression, ()> {
    let new_type = get_default_type_from_identifier(ty, ns)?;
    let type_size = new_type.get_type_size();

    if byte_array.len() * 8 > type_size as usize {
        ns.diagnostics.push(Diagnostic {
            level: Level::Error,
            ty: ErrorType::DeclarationError,
            loc: *loc,
            message: format!(
                "the provided literal requires {} bits, but the type only supports {}",
                byte_array.len() * 8,
                type_size
            ),
            notes: vec![],
        });
    }

    Ok(YulExpression::StringLiteral(*loc, byte_array, new_type))
}

fn resolve_variable_reference(
    id: &pt::Identifier,
    ns: &mut Namespace,
    symtable: &Symtable,
    context: &ExprContext,
) -> Result<YulExpression, ()> {
    if let Some(v) = symtable.find(&id.name) {
        match &v.usage_type {
            VariableUsage::YulLocalVariable => {
                return Ok(YulExpression::YulLocalVariable(id.loc, v.ty.clone(), v.pos))
            }
            VariableUsage::AnonymousReturnVariable => {
                unreachable!("Anonymous returns variables cannot be accessed from assembly blocks")
            }
            _ => {
                return Ok(YulExpression::SolidityLocalVariable(
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
                    Ok(YulExpression::ConstantVariable(
                        id.loc,
                        var.ty.clone(),
                        Some(*var_contract_no),
                        *var_no,
                    ))
                } else {
                    Ok(YulExpression::StorageVariable(
                        id.loc,
                        var.ty.clone(),
                        *var_contract_no,
                        *var_no,
                    ))
                }
            }
            Some(Symbol::Variable(_, None, var_no)) => {
                let var = &ns.constants[*var_no];
                Ok(YulExpression::ConstantVariable(
                    id.loc,
                    var.ty.clone(),
                    None,
                    *var_no,
                ))
            }
            None => {
                ns.diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!("'{}' not found", id.name),
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
        format!("'{}' not found", id.name),
    ));
    Err(())
}

pub(crate) fn resolve_function_call(
    function_table: &mut FunctionsTable,
    func_call: &YulFunctionCall,
    context: &ExprContext,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<YulExpression, ()> {
    if func_call.id.name.starts_with("verbatim") {
        ns.diagnostics.push(Diagnostic::error(
            func_call.id.loc,
            "verbatim functions are not yet supported in Solang".to_string(),
        ));
        return Err(());
    } else if yul_unsupported_builtin(func_call.id.name.as_str()) {
        ns.diagnostics.push(Diagnostic::error(
            func_call.id.loc,
            format!(
                "the internal EVM built-in '{}' is not yet supported",
                func_call.id.name
            ),
        ));
        return Err(());
    }
    let mut resolved_arguments: Vec<YulExpression> = Vec::with_capacity(func_call.arguments.len());
    for item in &func_call.arguments {
        let resolved_expr = resolve_yul_expression(item, context, symtable, function_table, ns)?;

        if let Some(diagnostic) = check_type(&resolved_expr, context, ns, symtable) {
            ns.diagnostics.push(diagnostic);
            return Err(());
        }

        resolved_arguments.push(resolved_expr);
    }

    if let Some(built_in) = parse_builtin_keyword(func_call.id.name.as_str()) {
        let prototype = &built_in.get_prototype_info();
        if !prototype.is_available(&ns.target) {
            ns.diagnostics.push(Diagnostic::error(
                func_call.loc,
                format!(
                    "builtin '{}' is not available for target {}. Please, open a GitHub issue \
                at https://github.com/hyperledger-labs/solang/issues \
                if there is need to support this function",
                    prototype.name, ns.target
                ),
            ));
            return Err(());
        }
        if prototype.no_args as usize != func_call.arguments.len() {
            ns.diagnostics.push(Diagnostic {
                level: Level::Error,
                ty: ErrorType::TypeError,
                loc: func_call.loc,
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

        let default_builtin_parameter = Parameter {
            loc: Loc::Builtin,
            id: None,
            ty: Type::Uint(256),
            ty_loc: None,
            indexed: false,
            readonly: false,
            recursive: false,
        };

        for item in &resolved_arguments {
            check_function_argument(&default_builtin_parameter, item, function_table, ns);
        }

        return Ok(YulExpression::BuiltInCall(
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
        let resolved_fn = Ok(YulExpression::FunctionCall(
            func_call.id.loc,
            fn_no,
            resolved_arguments,
            func.returns.clone(),
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
    parameter: &Parameter,
    argument: &YulExpression,
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
fn resolve_suffix_access(
    loc: &pt::Loc,
    expr: &pt::YulExpression,
    id: &Identifier,
    context: &ExprContext,
    symtable: &mut Symtable,
    function_table: &mut FunctionsTable,
    ns: &mut Namespace,
) -> Result<YulExpression, ()> {
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

    let resolved_expr = resolve_yul_expression(expr, context, symtable, function_table, ns)?;
    match resolved_expr {
        YulExpression::ConstantVariable(_, _, Some(_), _) => {
            ns.diagnostics.push(Diagnostic::error(
                resolved_expr.loc(),
                "the suffixes .offset and .slot can only be used in non-constant storage variables"
                    .to_string(),
            ));
            return Err(());
        }

        YulExpression::SolidityLocalVariable(
            _,
            Type::Array(_, ref dims),
            Some(StorageLocation::Calldata(_)),
            _,
        ) => {
            if dims.last() == Some(&ArrayLength::Dynamic)
                && id.name != "offset"
                && id.name != "length"
            {
                ns.diagnostics.push(Diagnostic::error(
                    resolved_expr.loc(),
                    "calldata variables only support '.offset' and '.length'".to_string(),
                ));
                return Err(());
            } else if matches!(dims.last(), Some(ArrayLength::Fixed(_))) {
                ns.diagnostics.push(Diagnostic::error(
                    resolved_expr.loc(),
                    format!(
                        "the given expression does not support '.{}' suffixes",
                        suffix_type.to_string()
                    ),
                ));
            }
        }

        YulExpression::SolidityLocalVariable(_, Type::InternalFunction { .. }, ..)
        | YulExpression::ConstantVariable(_, Type::InternalFunction { .. }, ..)
        | YulExpression::StorageVariable(_, Type::InternalFunction { .. }, ..) => {
            ns.diagnostics.push(Diagnostic::error(
                resolved_expr.loc(),
                "only variables of type external function pointer support suffixes".to_string(),
            ));
            return Err(());
        }

        YulExpression::SolidityLocalVariable(_, Type::ExternalFunction { .. }, ..) => {
            if id.name != "selector" && id.name != "address" {
                ns.diagnostics.push(Diagnostic::error(
                    id.loc,
                    "variables of type function pointer only support '.selector' and '.address' suffixes".to_string()
                ));
                return Err(());
            }
        }

        YulExpression::SolidityLocalVariable(_, _, Some(StorageLocation::Storage(_)), _)
        | YulExpression::StorageVariable(_, _, _, _) => {
            if id.name != "slot" && id.name != "offset" {
                ns.diagnostics.push(Diagnostic::error(
                    id.loc,
                    "state variables only support '.slot' and '.offset'".to_string(),
                ));
                return Err(());
            }
        }

        YulExpression::SuffixAccess(..) => {
            ns.diagnostics.push(Diagnostic::error(
                id.loc,
                "there cannot be multiple suffixes to a name".to_string(),
            ));
            return Err(());
        }

        YulExpression::BoolLiteral(..)
        | YulExpression::NumberLiteral(..)
        | YulExpression::StringLiteral(..)
        | YulExpression::YulLocalVariable(..)
        | YulExpression::SolidityLocalVariable(_, _, Some(StorageLocation::Memory(_)), ..)
        | YulExpression::SolidityLocalVariable(_, _, Some(StorageLocation::Calldata(_)), ..)
        | YulExpression::SolidityLocalVariable(_, _, None, ..)
        | YulExpression::BuiltInCall(..)
        | YulExpression::FunctionCall(..)
        | YulExpression::ConstantVariable(_, _, None, _) => {
            ns.diagnostics.push(Diagnostic::error(
                resolved_expr.loc(),
                format!(
                    "the given expression does not support '.{}' suffixes",
                    suffix_type.to_string()
                ),
            ));
            return Err(());
        }
    }

    Ok(YulExpression::SuffixAccess(
        *loc,
        Box::new(resolved_expr),
        suffix_type,
    ))
}

/// Check if an yul expression has been used correctly in a assignment or if the member access
/// has a valid expression given the context.
pub(crate) fn check_type(
    expr: &YulExpression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
) -> Option<Diagnostic> {
    if context.lvalue {
        match expr {
            YulExpression::SolidityLocalVariable(_, _, Some(StorageLocation::Storage(_)), ..)
            | YulExpression::StorageVariable(..) => {
                return Some(Diagnostic::error(
                    expr.loc(),
                    "storage variables cannot be assigned any value in assembly. You may use 'sstore()'".to_string()
                ));
            }

            YulExpression::StringLiteral(..)
            | YulExpression::NumberLiteral(..)
            | YulExpression::BoolLiteral(..)
            | YulExpression::ConstantVariable(..) => {
                return Some(Diagnostic::error(
                    expr.loc(),
                    "cannot assigned a value to a constant".to_string(),
                ));
            }

            YulExpression::BuiltInCall(..) | YulExpression::FunctionCall(..) => {
                return Some(Diagnostic::error(
                    expr.loc(),
                    "cannot assign a value to a function".to_string(),
                ));
            }

            YulExpression::SuffixAccess(_, member, YulSuffix::Length) => {
                return if matches!(
                    **member,
                    YulExpression::SolidityLocalVariable(
                        _,
                        _,
                        Some(StorageLocation::Calldata(_)),
                        _
                    )
                ) {
                    Some(Diagnostic::error(
                        expr.loc(),
                        "assignment to length is not implemented. If there is need for this feature, please file a Github issue \
                        at https://github.com/hyperledger-labs/solang/issues\
                        ".to_string(),
                    ))
                } else {
                    Some(Diagnostic::error(
                        expr.loc(),
                        "this expression does not support the '.length' suffix".to_string(),
                    ))
                }
            }

            YulExpression::SuffixAccess(_, member, YulSuffix::Offset) => {
                if !matches!(
                    **member,
                    YulExpression::SolidityLocalVariable(
                        _,
                        _,
                        Some(StorageLocation::Calldata(_)),
                        _
                    )
                ) {
                    return Some(Diagnostic::error(
                        expr.loc(),
                        "cannot assign a value to offset".to_string(),
                    ));
                }
            }
            YulExpression::SuffixAccess(_, exp, YulSuffix::Slot) => {
                if matches!(**exp, YulExpression::StorageVariable(..)) {
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
        YulExpression::SolidityLocalVariable(_, _, Some(StorageLocation::Storage(_)), ..)
        | YulExpression::StorageVariable(..) => {
            return Some(Diagnostic::error(
                expr.loc(),
                "Storage variables must be accessed with '.slot' or '.offset'".to_string(),
            ));
        }

        YulExpression::SolidityLocalVariable(
            _,
            Type::Array(_, ref dims),
            Some(StorageLocation::Calldata(_)),
            ..,
        ) => {
            if dims.last() == Some(&ArrayLength::Dynamic) {
                return Some(Diagnostic::error(
                    expr.loc(),
                    "Calldata arrays must be accessed with '.offset', '.length' and the 'calldatacopy' function".to_string()
                ));
            }
        }

        _ => (),
    }

    None
}
