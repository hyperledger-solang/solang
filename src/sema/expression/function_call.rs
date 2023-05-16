// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{
    ArrayLength, Builtin, CallArgs, CallTy, Expression, Function, Mutability, Namespace,
    RetrieveType, StructType, Symbol, Type,
};
use crate::sema::contracts::is_base;
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::constructor::{deprecated_constructor_arguments, new};
use crate::sema::expression::literals::{named_struct_literal, struct_literal};
use crate::sema::expression::resolve_expression::expression;
use crate::sema::expression::{ExprContext, ResolveTo};
use crate::sema::format::string_format;
use crate::sema::symtable::Symtable;
use crate::sema::unused_variable::check_function_call;
use crate::sema::{builtin, using};
use crate::Target;
use solang_parser::diagnostics::Diagnostic;
use solang_parser::pt;
use solang_parser::pt::{CodeLocation, Loc, Visibility};
use std::collections::HashMap;

/// Resolve a function call via function type
/// Function types do not have names so call cannot be using named parameters
pub(super) fn call_function_type(
    loc: &pt::Loc,
    expr: &pt::Expression,
    args: &[pt::Expression],
    call_args: &[&pt::NamedArgument],
    call_args_loc: Option<pt::Loc>,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let mut function = expression(expr, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;

    let mut ty = function.ty();

    match ty {
        Type::StorageRef(_, real_ty) | Type::Ref(real_ty) => {
            ty = *real_ty;
            function = function.cast(&expr.loc(), &ty, true, ns, diagnostics)?;
        }
        _ => (),
    };

    if let Type::InternalFunction {
        params, returns, ..
    } = ty
    {
        if let Some(loc) = call_args_loc {
            diagnostics.push(Diagnostic::error(
                loc,
                "call arguments not permitted for internal calls".to_string(),
            ));
        }

        if params.len() != args.len() {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "function expects {} arguments, {} provided",
                    params.len(),
                    args.len()
                ),
            ));
            return Err(());
        }

        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for (i, arg) in args.iter().enumerate() {
            let arg = expression(
                arg,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&params[i]),
            )?;

            cast_args.push(arg.cast(&arg.loc(), &params[i], true, ns, diagnostics)?);
        }

        Ok(Expression::InternalFunctionCall {
            loc: *loc,
            returns: if returns.is_empty() || resolve_to == ResolveTo::Discard {
                vec![Type::Void]
            } else {
                returns
            },
            function: Box::new(function),
            args: cast_args,
        })
    } else if let Type::ExternalFunction {
        returns,
        params,
        mutability,
    } = ty
    {
        let call_args = parse_call_args(loc, call_args, true, context, ns, symtable, diagnostics)?;

        if let Some(value) = &call_args.value {
            if !value.const_zero(ns) && !matches!(mutability, Mutability::Payable(_)) {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "sending value to function type '{}' which is not payable",
                        function.ty().to_string(ns),
                    ),
                ));
                return Err(());
            }
        }

        if params.len() != args.len() {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "function expects {} arguments, {} provided",
                    params.len(),
                    args.len()
                ),
            ));
            return Err(());
        }

        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for (i, arg) in args.iter().enumerate() {
            let arg = expression(
                arg,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&params[i]),
            )?;

            cast_args.push(arg.cast(&arg.loc(), &params[i], true, ns, diagnostics)?);
        }

        Ok(Expression::ExternalFunctionCall {
            loc: *loc,
            returns: if returns.is_empty() || resolve_to == ResolveTo::Discard {
                vec![Type::Void]
            } else {
                returns
            },
            function: Box::new(function),
            args: cast_args,
            call_args,
        })
    } else {
        diagnostics.push(Diagnostic::error(
            *loc,
            "expression is not a function".to_string(),
        ));
        Err(())
    }
}

/// Create a list of functions that can be called in this context. If global is true, then
/// include functions outside of contracts
pub fn available_functions(
    name: &str,
    global: bool,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &Namespace,
) -> Vec<usize> {
    let mut list = Vec::new();

    if global {
        if let Some(Symbol::Function(v)) =
            ns.function_symbols.get(&(file_no, None, name.to_owned()))
        {
            list.extend(v.iter().map(|(_, func_no)| *func_no));
        }
    }

    if let Some(contract_no) = contract_no {
        list.extend(
            ns.contracts[contract_no]
                .all_functions
                .keys()
                .filter_map(|func_no| {
                    if ns.functions[*func_no].name == name && ns.functions[*func_no].has_body {
                        Some(*func_no)
                    } else {
                        None
                    }
                }),
        );
    }

    list
}

/// Create a list of functions that can be called via super
pub fn available_super_functions(name: &str, contract_no: usize, ns: &Namespace) -> Vec<usize> {
    let mut list = Vec::new();

    for base_contract_no in ns.contract_bases(contract_no).into_iter().rev() {
        if base_contract_no == contract_no {
            continue;
        }

        list.extend(
            ns.contracts[base_contract_no]
                .all_functions
                .keys()
                .filter_map(|func_no| {
                    let func = &ns.functions[*func_no];

                    if func.name == name && func.has_body {
                        Some(*func_no)
                    } else {
                        None
                    }
                }),
        );
    }

    list
}

/// Resolve a function call with positional arguments
pub fn function_call_pos_args(
    loc: &pt::Loc,
    id: &pt::Identifier,
    func_ty: pt::FunctionTy,
    args: &[pt::Expression],
    function_nos: Vec<usize>,
    virtual_call: bool,
    context: &ExprContext,
    ns: &mut Namespace,
    resolve_to: ResolveTo,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let mut name_matches = 0;
    let mut errors = Diagnostics::default();

    // Try to resolve as a function call
    for function_no in &function_nos {
        let func = &ns.functions[*function_no];

        if func.ty != func_ty {
            continue;
        }

        name_matches += 1;

        let params_len = func.params.len();

        if params_len != args.len() {
            errors.push(Diagnostic::error(
                *loc,
                format!(
                    "{} expects {} arguments, {} provided",
                    func.ty,
                    params_len,
                    args.len()
                ),
            ));
            continue;
        }

        let mut matches = true;
        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for (i, arg) in args.iter().enumerate() {
            let ty = ns.functions[*function_no].params[i].ty.clone();

            matches &=
                evaluate_argument(arg, context, ns, symtable, &ty, &mut errors, &mut cast_args);
        }

        if !matches {
            if function_nos.len() > 1 && diagnostics.extend_non_casting(&errors) {
                return Err(());
            }

            continue;
        }

        match resolve_internal_call(
            loc,
            *function_no,
            context,
            resolve_to,
            virtual_call,
            cast_args,
            ns,
            &mut errors,
        ) {
            Some(resolved_call) => return Ok(resolved_call),
            None => continue,
        }
    }

    match name_matches {
        0 => {
            if func_ty == pt::FunctionTy::Modifier {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!("unknown modifier '{}'", id.name),
                ));
            } else {
                diagnostics.push(Diagnostic::error(
                    id.loc,
                    format!("unknown {} or type '{}'", func_ty, id.name),
                ));
            }
        }
        1 => diagnostics.extend(errors),
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!("cannot find overloaded {func_ty} which matches signature"),
            ));
        }
    }

    Err(())
}

/// Resolve a function call with named arguments
pub(super) fn function_call_named_args(
    loc: &pt::Loc,
    id: &pt::Identifier,
    args: &[pt::NamedArgument],
    function_nos: Vec<usize>,
    virtual_call: bool,
    context: &ExprContext,
    resolve_to: ResolveTo,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let mut arguments = HashMap::new();

    for arg in args {
        if arguments.contains_key(arg.name.name.as_str()) {
            diagnostics.push(Diagnostic::error(
                arg.name.loc,
                format!("duplicate argument with name '{}'", arg.name.name),
            ));

            let _ = expression(
                &arg.expr,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Unknown,
            );
        }

        arguments.insert(arg.name.name.as_str(), &arg.expr);
    }
    // Try to resolve as a function call
    let mut errors = Diagnostics::default();

    // Try to resolve as a function call
    for function_no in &function_nos {
        let func = &ns.functions[*function_no];

        if func.ty != pt::FunctionTy::Function {
            continue;
        }

        let unnamed_params = func.params.iter().filter(|p| p.id.is_none()).count();
        let params_len = func.params.len();
        let mut matches = true;

        if unnamed_params > 0 {
            errors.push(Diagnostic::cast_error_with_note(
                *loc,
                format!(
                    "function cannot be called with named arguments as {unnamed_params} of its parameters do not have names"
                ),
                func.loc,
                format!("definition of {}", func.name),
            ));
            matches = false;
        } else if params_len != args.len() {
            errors.push(Diagnostic::cast_error(
                *loc,
                format!(
                    "function expects {} arguments, {} provided",
                    params_len,
                    args.len()
                ),
            ));
            matches = false;
        }

        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for i in 0..params_len {
            let param = &ns.functions[*function_no].params[i];
            if param.id.is_none() {
                continue;
            }
            let arg = match arguments.get(param.name_as_str()) {
                Some(a) => a,
                None => {
                    matches = false;
                    diagnostics.push(Diagnostic::cast_error(
                        *loc,
                        format!(
                            "missing argument '{}' to function '{}'",
                            param.name_as_str(),
                            id.name,
                        ),
                    ));
                    continue;
                }
            };

            let ty = param.ty.clone();

            matches &=
                evaluate_argument(arg, context, ns, symtable, &ty, &mut errors, &mut cast_args);
        }

        if !matches {
            if diagnostics.extend_non_casting(&errors) {
                return Err(());
            }
            continue;
        }

        match resolve_internal_call(
            loc,
            *function_no,
            context,
            resolve_to,
            virtual_call,
            cast_args,
            ns,
            &mut errors,
        ) {
            Some(resolved_call) => return Ok(resolved_call),
            None => continue,
        }
    }

    match function_nos.len() {
        0 => {
            diagnostics.push(Diagnostic::error(
                id.loc,
                format!("unknown function or type '{}'", id.name),
            ));
        }
        1 => diagnostics.extend(errors),
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "cannot find overloaded function which matches signature".to_string(),
            ));
        }
    }

    Err(())
}

/// Check if the function is a method of a variable
/// Returns:
/// 1. Err, when there is an error
/// 2. Ok(Some()), when we have indeed received a method of a variable
/// 3. Ok(None), when we have not received a function that is a method of a variable
fn try_namespace(
    loc: &pt::Loc,
    var: &pt::Expression,
    func: &pt::Identifier,
    args: &[pt::Expression],
    call_args_loc: Option<pt::Loc>,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Option<Expression>, ()> {
    if let pt::Expression::Variable(namespace) = var {
        if builtin::is_builtin_call(Some(&namespace.name), &func.name, ns) {
            if let Some(loc) = call_args_loc {
                diagnostics.push(Diagnostic::error(
                    loc,
                    "call arguments not allowed on builtins".to_string(),
                ));
                return Err(());
            }

            return Ok(Some(builtin::resolve_namespace_call(
                loc,
                &namespace.name,
                &func.name,
                args,
                context,
                ns,
                symtable,
                diagnostics,
            )?));
        }

        // is it a call to super
        if namespace.name == "super" {
            if let Some(cur_contract_no) = context.contract_no {
                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on super calls".to_string(),
                    ));
                    return Err(());
                }

                return Ok(Some(function_call_pos_args(
                    loc,
                    func,
                    pt::FunctionTy::Function,
                    args,
                    available_super_functions(&func.name, cur_contract_no, ns),
                    false,
                    context,
                    ns,
                    resolve_to,
                    symtable,
                    diagnostics,
                )?));
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "super not available outside contracts".to_string(),
                ));
                return Err(());
            }
        }

        // library or base contract call
        if let Some(call_contract_no) = ns.resolve_contract(context.file_no, namespace) {
            if ns.contracts[call_contract_no].is_library() {
                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on library calls".to_string(),
                    ));
                    return Err(());
                }

                return Ok(Some(function_call_pos_args(
                    loc,
                    func,
                    pt::FunctionTy::Function,
                    args,
                    available_functions(
                        &func.name,
                        false,
                        context.file_no,
                        Some(call_contract_no),
                        ns,
                    ),
                    true,
                    context,
                    ns,
                    resolve_to,
                    symtable,
                    diagnostics,
                )?));
            }

            // is a base contract of us
            if let Some(contract_no) = context.contract_no {
                if is_base(call_contract_no, contract_no, ns) {
                    if let Some(loc) = call_args_loc {
                        diagnostics.push(Diagnostic::error(
                            loc,
                            "call arguments not allowed on internal calls".to_string(),
                        ));
                        return Err(());
                    }

                    return Ok(Some(function_call_pos_args(
                        loc,
                        func,
                        pt::FunctionTy::Function,
                        args,
                        available_functions(
                            &func.name,
                            false,
                            context.file_no,
                            Some(call_contract_no),
                            ns,
                        ),
                        false,
                        context,
                        ns,
                        resolve_to,
                        symtable,
                        diagnostics,
                    )?));
                } else {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        "function calls via contract name are only valid for base contracts".into(),
                    ));
                }
            }
        }
    }

    Ok(None)
}

/// Check if the function is a method of a storage reference
/// Returns:
/// 1. Err, when there is an error
/// 2. Ok(Some()), when we have indeed received a method of a storage reference
/// 3. Ok(None), when we have not received a function that is a method of a storage reference
fn try_storage_reference(
    loc: &pt::Loc,
    var_expr: &Expression,
    func: &pt::Identifier,
    args: &[pt::Expression],
    context: &ExprContext,
    diagnostics: &mut Diagnostics,
    call_args_loc: Option<pt::Loc>,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    resolve_to: &ResolveTo,
) -> Result<Option<Expression>, ()> {
    if let Type::StorageRef(immutable, ty) = &var_expr.ty() {
        match ty.as_ref() {
            Type::Array(_, dim) => {
                if *immutable {
                    if let Some(function_no) = context.function_no {
                        if !ns.functions[function_no].is_constructor() {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                "cannot call method on immutable array outside of constructor"
                                    .to_string(),
                            ));
                            return Err(());
                        }
                    }
                }

                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on arrays".to_string(),
                    ));
                    return Err(());
                }

                if func.name == "push" {
                    if matches!(dim.last(), Some(ArrayLength::Fixed(_))) {
                        diagnostics.push(Diagnostic::error(
                            func.loc,
                            "method 'push()' not allowed on fixed length array".to_string(),
                        ));
                        return Err(());
                    }

                    let elem_ty = ty.array_elem();
                    let mut builtin_args = vec![var_expr.clone()];

                    let ret_ty = match args.len() {
                        1 => {
                            let expr = expression(
                                &args[0],
                                context,
                                ns,
                                symtable,
                                diagnostics,
                                ResolveTo::Type(&elem_ty),
                            )?;

                            builtin_args.push(expr.cast(
                                &args[0].loc(),
                                &elem_ty,
                                true,
                                ns,
                                diagnostics,
                            )?);

                            Type::Void
                        }
                        0 => {
                            if elem_ty.is_reference_type(ns) {
                                Type::StorageRef(false, Box::new(elem_ty))
                            } else {
                                elem_ty
                            }
                        }
                        _ => {
                            diagnostics.push(Diagnostic::error(
                                func.loc,
                                "method 'push()' takes at most 1 argument".to_string(),
                            ));
                            return Err(());
                        }
                    };

                    return Ok(Some(Expression::Builtin {
                        loc: func.loc,
                        tys: vec![ret_ty],
                        kind: Builtin::ArrayPush,
                        args: builtin_args,
                    }));
                }
                if func.name == "pop" {
                    if matches!(dim.last(), Some(ArrayLength::Fixed(_))) {
                        diagnostics.push(Diagnostic::error(
                            func.loc,
                            "method 'pop()' not allowed on fixed length array".to_string(),
                        ));

                        return Err(());
                    }

                    if !args.is_empty() {
                        diagnostics.push(Diagnostic::error(
                            func.loc,
                            "method 'pop()' does not take any arguments".to_string(),
                        ));
                        return Err(());
                    }

                    let storage_elem = ty.storage_array_elem();
                    let elem_ty = storage_elem.deref_any();

                    let return_ty = if *resolve_to == ResolveTo::Discard {
                        Type::Void
                    } else {
                        elem_ty.clone()
                    };

                    return Ok(Some(Expression::Builtin {
                        loc: func.loc,
                        tys: vec![return_ty],
                        kind: Builtin::ArrayPop,
                        args: vec![var_expr.clone()],
                    }));
                }
            }
            Type::DynamicBytes => {
                if *immutable {
                    if let Some(function_no) = context.function_no {
                        if !ns.functions[function_no].is_constructor() {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                "cannot call method on immutable bytes outside of constructor"
                                    .to_string(),
                            ));
                            return Err(());
                        }
                    }
                }

                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on bytes".to_string(),
                    ));
                    return Err(());
                }

                if func.name == "push" {
                    let mut builtin_args = vec![var_expr.clone()];

                    let elem_ty = Type::Bytes(1);

                    let ret_ty = match args.len() {
                        1 => {
                            let expr = expression(
                                &args[0],
                                context,
                                ns,
                                symtable,
                                diagnostics,
                                ResolveTo::Type(&elem_ty),
                            )?;

                            builtin_args.push(expr.cast(
                                &args[0].loc(),
                                &elem_ty,
                                true,
                                ns,
                                diagnostics,
                            )?);

                            Type::Void
                        }
                        0 => elem_ty,
                        _ => {
                            diagnostics.push(Diagnostic::error(
                                func.loc,
                                "method 'push()' takes at most 1 argument".to_string(),
                            ));
                            return Err(());
                        }
                    };
                    return Ok(Some(Expression::Builtin {
                        loc: func.loc,
                        tys: vec![ret_ty],
                        kind: Builtin::ArrayPush,
                        args: builtin_args,
                    }));
                }

                if func.name == "pop" {
                    if !args.is_empty() {
                        diagnostics.push(Diagnostic::error(
                            func.loc,
                            "method 'pop()' does not take any arguments".to_string(),
                        ));
                        return Err(());
                    }

                    return Ok(Some(Expression::Builtin {
                        loc: func.loc,
                        tys: vec![Type::Bytes(1)],
                        kind: Builtin::ArrayPop,
                        args: vec![var_expr.clone()],
                    }));
                }
            }
            _ => {}
        }
    }

    Ok(None)
}

/// Check if we can resolve the call with ns.resolve_type
/// Returns:
/// 1. Err, when there is an error
/// 2. Ok(Some()), when we have indeed received a method that has correctly been resolved with
///    ns.resolve_type
/// 3. Ok(None), when the function we have received could not be resolved with ns.resolve_type
fn try_user_type(
    loc: &pt::Loc,
    var: &pt::Expression,
    func: &pt::Identifier,
    args: &[pt::Expression],
    call_args_loc: Option<pt::Loc>,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Option<Expression>, ()> {
    if let Ok(Type::UserType(no)) = ns.resolve_type(
        context.file_no,
        context.contract_no,
        false,
        var,
        &mut Diagnostics::default(),
    ) {
        if let Some(loc) = call_args_loc {
            diagnostics.push(Diagnostic::error(
                loc,
                "call arguments not allowed on builtins".to_string(),
            ));
            return Err(());
        }

        let elem_ty = ns.user_types[no].ty.clone();
        let user_ty = Type::UserType(no);

        if func.name == "unwrap" {
            return if args.len() != 1 {
                diagnostics.push(Diagnostic::error(
                    func.loc,
                    "method 'unwrap()' takes one argument".to_string(),
                ));
                Err(())
            } else {
                let expr = expression(
                    &args[0],
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&user_ty),
                )?;

                Ok(Some(Expression::Builtin {
                    loc: *loc,
                    tys: vec![elem_ty],
                    kind: Builtin::UserTypeUnwrap,
                    args: vec![expr.cast(&expr.loc(), &user_ty, true, ns, diagnostics)?],
                }))
            };
        } else if func.name == "wrap" {
            return if args.len() != 1 {
                diagnostics.push(Diagnostic::error(
                    func.loc,
                    "method 'wrap()' takes one argument".to_string(),
                ));
                Err(())
            } else {
                let expr = expression(
                    &args[0],
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&elem_ty),
                )?;

                Ok(Some(Expression::Builtin {
                    loc: *loc,
                    tys: vec![user_ty],
                    kind: Builtin::UserTypeWrap,
                    args: vec![expr.cast(&expr.loc(), &elem_ty, true, ns, diagnostics)?],
                }))
            };
        }
    }

    Ok(None)
}

/// Check if the function call is to a type's method
/// Returns:
/// 1. Err, when there is an error
/// 2. Ok(Some()), when we have indeed received a method of a type
/// 3. Ok(None), when we have received a function that is not a method of a type
fn try_type_method(
    loc: &pt::Loc,
    func: &pt::Identifier,
    var: &pt::Expression,
    args: &[pt::Expression],
    call_args: &[&pt::NamedArgument],
    call_args_loc: Option<pt::Loc>,
    context: &ExprContext,
    var_expr: &Expression,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Option<Expression>, ()> {
    let var_ty = var_expr.ty();

    match var_ty.deref_any() {
        Type::Bytes(..) | Type::String if func.name == "format" => {
            return if let pt::Expression::StringLiteral(bs) = var {
                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on builtins".to_string(),
                    ));
                    return Err(());
                }

                Ok(Some(string_format(
                    loc,
                    bs,
                    args,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                )?))
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "format only allowed on string literals".to_string(),
                ));
                Err(())
            };
        }

        Type::Array(..) | Type::DynamicBytes => {
            if func.name == "push" {
                let elem_ty = var_ty.array_elem();

                let val = match args.len() {
                    0 => {
                        return Ok(Some(Expression::Builtin {
                            loc: *loc,
                            tys: vec![elem_ty.clone()],
                            kind: Builtin::ArrayPush,
                            args: vec![var_expr.clone()],
                        }));
                    }
                    1 => {
                        let val_expr = expression(
                            &args[0],
                            context,
                            ns,
                            symtable,
                            diagnostics,
                            ResolveTo::Type(&elem_ty),
                        )?;

                        val_expr.cast(&args[0].loc(), &elem_ty, true, ns, diagnostics)?
                    }
                    _ => {
                        diagnostics.push(Diagnostic::error(
                            func.loc,
                            "method 'push()' takes at most 1 argument".to_string(),
                        ));
                        return Err(());
                    }
                };

                return Ok(Some(Expression::Builtin {
                    loc: *loc,
                    tys: vec![elem_ty.clone()],
                    kind: Builtin::ArrayPush,
                    args: vec![var_expr.clone(), val],
                }));
            }
            if func.name == "pop" {
                if !args.is_empty() {
                    diagnostics.push(Diagnostic::error(
                        func.loc,
                        "method 'pop()' does not take any arguments".to_string(),
                    ));
                    return Err(());
                }

                let elem_ty = match &var_ty {
                    Type::Array(ty, _) => ty,
                    Type::DynamicBytes => &Type::Uint(8),
                    _ => unreachable!(),
                };

                return Ok(Some(Expression::Builtin {
                    loc: *loc,
                    tys: vec![elem_ty.clone()],
                    kind: Builtin::ArrayPop,
                    args: vec![var_expr.clone()],
                }));
            }
        }

        Type::Contract(ext_contract_no) => {
            let call_args =
                parse_call_args(loc, call_args, true, context, ns, symtable, diagnostics)?;

            let mut errors = Diagnostics::default();
            let mut name_matches: Vec<usize> = Vec::new();

            for function_no in ns.contracts[*ext_contract_no].all_functions.keys() {
                if func.name != ns.functions[*function_no].name
                    || ns.functions[*function_no].ty != pt::FunctionTy::Function
                {
                    continue;
                }

                name_matches.push(*function_no);
            }

            for function_no in &name_matches {
                let params_len = ns.functions[*function_no].params.len();

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
                for (i, arg) in args.iter().enumerate() {
                    let ty = ns.functions[*function_no].params[i].ty.clone();

                    let arg = match expression(
                        arg,
                        context,
                        ns,
                        symtable,
                        &mut errors,
                        ResolveTo::Type(&ty),
                    ) {
                        Ok(e) => e,
                        Err(_) => {
                            matches = false;
                            continue;
                        }
                    };

                    match arg.cast(&arg.loc(), &ty, true, ns, &mut errors) {
                        Ok(expr) => cast_args.push(expr),
                        Err(()) => {
                            matches = false;
                            continue;
                        }
                    }
                }

                if matches {
                    if !ns.functions[*function_no].is_public() {
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!("function '{}' is not 'public' or 'external'", func.name),
                        ));
                        return Err(());
                    }

                    if let Some(value) = &call_args.value {
                        if !value.const_zero(ns) && !ns.functions[*function_no].is_payable() {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                format!(
                                    "sending value to function '{}' which is not payable",
                                    func.name
                                ),
                            ));
                            return Err(());
                        }
                    }

                    let func = &ns.functions[*function_no];
                    let returns = function_returns(func, resolve_to);
                    let ty = function_type(func, true, resolve_to);

                    return Ok(Some(Expression::ExternalFunctionCall {
                        loc: *loc,
                        returns,
                        function: Box::new(Expression::ExternalFunction {
                            loc: *loc,
                            ty,
                            function_no: *function_no,
                            address: Box::new(var_expr.cast(
                                &var.loc(),
                                &Type::Contract(func.contract_no.unwrap()),
                                true,
                                ns,
                                diagnostics,
                            )?),
                        }),
                        args: cast_args,
                        call_args,
                    }));
                } else if name_matches.len() > 1 && diagnostics.extend_non_casting(&errors) {
                    return Err(());
                }
            }

            // what about call args
            match using::try_resolve_using_call(
                loc,
                func,
                var_expr,
                context,
                args,
                symtable,
                diagnostics,
                ns,
                resolve_to,
            ) {
                Ok(Some(expr)) => {
                    return Ok(Some(expr));
                }
                Ok(None) => (),
                Err(_) => {
                    return Err(());
                }
            }

            if name_matches.len() == 1 {
                diagnostics.extend(errors);
            } else if name_matches.len() != 1 {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "cannot find overloaded function which matches signature".to_string(),
                ));
            }

            return Err(());
        }

        Type::Address(is_payable) => {
            if func.name == "transfer" || func.name == "send" {
                if !is_payable {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "method '{}' available on type 'address payable' not 'address'",
                            func.name,
                        ),
                    ));

                    return Err(());
                }

                if args.len() != 1 {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "'{}' expects 1 argument, {} provided",
                            func.name,
                            args.len()
                        ),
                    ));

                    return Err(());
                }

                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        format!("call arguments not allowed on '{}'", func.name),
                    ));
                    return Err(());
                }

                let expr = expression(
                    &args[0],
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&Type::Value),
                )?;

                let address =
                    var_expr.cast(&var_expr.loc(), var_ty.deref_any(), true, ns, diagnostics)?;

                let value = expr.cast(&args[0].loc(), &Type::Value, true, ns, diagnostics)?;

                return if func.name == "transfer" {
                    Ok(Some(Expression::Builtin {
                        loc: *loc,
                        tys: vec![Type::Void],
                        kind: Builtin::PayableTransfer,
                        args: vec![address, value],
                    }))
                } else {
                    Ok(Some(Expression::Builtin {
                        loc: *loc,
                        tys: vec![Type::Bool],
                        kind: Builtin::PayableSend,
                        args: vec![address, value],
                    }))
                };
            }

            let ty = match func.name.as_str() {
                "call" => Some(CallTy::Regular),
                "delegatecall" if ns.target == Target::EVM => Some(CallTy::Delegate),
                "staticcall" if ns.target == Target::EVM => Some(CallTy::Static),
                _ => None,
            };

            if let Some(ty) = ty {
                let call_args =
                    parse_call_args(loc, call_args, true, context, ns, symtable, diagnostics)?;

                if ty != CallTy::Regular && call_args.value.is_some() {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!("'{}' cannot have value specified", func.name,),
                    ));

                    return Err(());
                }

                if args.len() != 1 {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "'{}' expects 1 argument, {} provided",
                            func.name,
                            args.len()
                        ),
                    ));

                    return Err(());
                }

                let args = expression(
                    &args[0],
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&Type::DynamicBytes),
                )?;

                let mut args_ty = args.ty();

                match args_ty.deref_any() {
                    Type::DynamicBytes => (),
                    Type::Bytes(_) => {
                        args_ty = Type::DynamicBytes;
                    }
                    Type::Array(..) | Type::Struct(..) if !args_ty.is_dynamic(ns) => {}
                    _ => {
                        diagnostics.push(Diagnostic::error(
                            args.loc(),
                            format!("'{}' is not fixed length type", args_ty.to_string(ns),),
                        ));

                        return Err(());
                    }
                }

                let args = args.cast(&args.loc(), args_ty.deref_any(), true, ns, diagnostics)?;

                return Ok(Some(Expression::ExternalFunctionCallRaw {
                    loc: *loc,
                    ty,
                    args: Box::new(args),
                    address: Box::new(var_expr.cast(
                        &var_expr.loc(),
                        &Type::Address(*is_payable),
                        true,
                        ns,
                        diagnostics,
                    )?),
                    call_args,
                }));
            }
        }

        _ => (),
    }

    Ok(None)
}

/// Resolve a method call with positional arguments
pub(super) fn method_call_pos_args(
    loc: &pt::Loc,
    var: &pt::Expression,
    func: &pt::Identifier,
    args: &[pt::Expression],
    call_args: &[&pt::NamedArgument],
    call_args_loc: Option<pt::Loc>,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    if let Some(resolved_call) = try_namespace(
        loc,
        var,
        func,
        args,
        call_args_loc,
        context,
        ns,
        symtable,
        diagnostics,
        resolve_to,
    )? {
        return Ok(resolved_call);
    }

    if let Some(resolved_call) = try_user_type(
        loc,
        var,
        func,
        args,
        call_args_loc,
        context,
        ns,
        symtable,
        diagnostics,
    )? {
        return Ok(resolved_call);
    }

    if let Some(mut path) = ns.expr_to_identifier_path(var) {
        path.identifiers.push(func.clone());

        if let Ok(list) = ns.resolve_free_function_with_namespace(
            context.file_no,
            &path,
            &mut Diagnostics::default(),
        ) {
            if let Some(loc) = call_args_loc {
                diagnostics.push(Diagnostic::error(
                    loc,
                    "call arguments not allowed on internal calls".to_string(),
                ));
            }

            return function_call_pos_args(
                loc,
                func,
                pt::FunctionTy::Function,
                args,
                list.iter().map(|(_, no)| *no).collect(),
                false,
                context,
                ns,
                resolve_to,
                symtable,
                diagnostics,
            );
        }
    }

    let var_expr = expression(var, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;

    if let Some(expr) =
        builtin::resolve_method_call(&var_expr, func, args, context, ns, symtable, diagnostics)?
    {
        return Ok(expr);
    }

    if let Some(resolved_call) = try_storage_reference(
        loc,
        &var_expr,
        func,
        args,
        context,
        diagnostics,
        call_args_loc,
        ns,
        symtable,
        &resolve_to,
    )? {
        return Ok(resolved_call);
    }

    if let Some(resolved_call) = try_type_method(
        loc,
        func,
        var,
        args,
        call_args,
        call_args_loc,
        context,
        &var_expr,
        ns,
        symtable,
        diagnostics,
        resolve_to,
    )? {
        return Ok(resolved_call);
    }

    // resolve it using library extension
    match using::try_resolve_using_call(
        loc,
        func,
        &var_expr,
        context,
        args,
        symtable,
        diagnostics,
        ns,
        resolve_to,
    ) {
        Ok(Some(expr)) => {
            return Ok(expr);
        }
        Ok(None) => (),
        Err(_) => {
            return Err(());
        }
    }

    diagnostics.push(Diagnostic::error(
        func.loc,
        format!("method '{}' does not exist", func.name),
    ));

    Err(())
}

pub(super) fn method_call_named_args(
    loc: &pt::Loc,
    var: &pt::Expression,
    func_name: &pt::Identifier,
    args: &[pt::NamedArgument],
    call_args: &[&pt::NamedArgument],
    call_args_loc: Option<pt::Loc>,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    if let pt::Expression::Variable(namespace) = var {
        // is it a call to super
        if namespace.name == "super" {
            if let Some(cur_contract_no) = context.contract_no {
                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on super calls".to_string(),
                    ));
                    return Err(());
                }

                return function_call_named_args(
                    loc,
                    func_name,
                    args,
                    available_super_functions(&func_name.name, cur_contract_no, ns),
                    false,
                    context,
                    resolve_to,
                    ns,
                    symtable,
                    diagnostics,
                );
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "super not available outside contracts".to_string(),
                ));
                return Err(());
            }
        }

        // library or base contract call
        if let Some(call_contract_no) = ns.resolve_contract(context.file_no, namespace) {
            if ns.contracts[call_contract_no].is_library() {
                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not allowed on library calls".to_string(),
                    ));
                    return Err(());
                }

                return function_call_named_args(
                    loc,
                    func_name,
                    args,
                    available_functions(
                        &func_name.name,
                        false,
                        context.file_no,
                        Some(call_contract_no),
                        ns,
                    ),
                    true,
                    context,
                    resolve_to,
                    ns,
                    symtable,
                    diagnostics,
                );
            }

            // is a base contract of us
            if let Some(contract_no) = context.contract_no {
                if is_base(call_contract_no, contract_no, ns) {
                    if let Some(loc) = call_args_loc {
                        diagnostics.push(Diagnostic::error(
                            loc,
                            "call arguments not allowed on internal calls".to_string(),
                        ));
                        return Err(());
                    }

                    return function_call_named_args(
                        loc,
                        func_name,
                        args,
                        available_functions(
                            &func_name.name,
                            false,
                            context.file_no,
                            Some(call_contract_no),
                            ns,
                        ),
                        false,
                        context,
                        resolve_to,
                        ns,
                        symtable,
                        diagnostics,
                    );
                } else {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        "function calls via contract name are only valid for base contracts".into(),
                    ));
                }
            }
        }
    }

    if let Some(mut path) = ns.expr_to_identifier_path(var) {
        path.identifiers.push(func_name.clone());

        if let Ok(list) = ns.resolve_free_function_with_namespace(
            context.file_no,
            &path,
            &mut Diagnostics::default(),
        ) {
            if let Some(loc) = call_args_loc {
                diagnostics.push(Diagnostic::error(
                    loc,
                    "call arguments not allowed on internal calls".to_string(),
                ));
            }

            return function_call_named_args(
                loc,
                func_name,
                args,
                list.iter().map(|(_, no)| *no).collect(),
                false,
                context,
                resolve_to,
                ns,
                symtable,
                diagnostics,
            );
        }
    }

    let var_expr = expression(var, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
    let var_ty = var_expr.ty();

    if let Type::Contract(external_contract_no) = &var_ty.deref_any() {
        let call_args = parse_call_args(loc, call_args, true, context, ns, symtable, diagnostics)?;

        let mut arguments = HashMap::new();

        // check if the arguments are not garbage
        for arg in args {
            if arguments.contains_key(arg.name.name.as_str()) {
                diagnostics.push(Diagnostic::error(
                    arg.name.loc,
                    format!("duplicate argument with name '{}'", arg.name.name),
                ));

                let _ = expression(
                    &arg.expr,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Unknown,
                );

                continue;
            }

            arguments.insert(arg.name.name.as_str(), &arg.expr);
        }

        let mut errors = Diagnostics::default();
        let mut name_matches: Vec<usize> = Vec::new();

        // function call
        for function_no in ns.contracts[*external_contract_no].all_functions.keys() {
            if ns.functions[*function_no].name != func_name.name
                || ns.functions[*function_no].ty != pt::FunctionTy::Function
            {
                continue;
            }

            name_matches.push(*function_no);
        }

        for function_no in &name_matches {
            let func = &ns.functions[*function_no];

            let unnamed_params = func.params.iter().filter(|p| p.id.is_none()).count();
            let params_len = func.params.len();

            let mut matches = true;

            if unnamed_params > 0 {
                errors.push(Diagnostic::cast_error_with_note(
                    *loc,
                    format!(
                        "function cannot be called with named arguments as {unnamed_params} of its parameters do not have names"
                    ),
                    func.loc,
                    format!("definition of {}", func.name),
                ));
                matches = false;
            } else if params_len != args.len() {
                errors.push(Diagnostic::cast_error(
                    *loc,
                    format!(
                        "function expects {} arguments, {} provided",
                        params_len,
                        args.len()
                    ),
                ));
                matches = false;
            }
            let mut cast_args = Vec::new();

            for i in 0..params_len {
                let param = ns.functions[*function_no].params[i].clone();
                if param.id.is_none() {
                    continue;
                }

                let arg = match arguments.get(param.name_as_str()) {
                    Some(a) => a,
                    None => {
                        matches = false;
                        diagnostics.push(Diagnostic::cast_error(
                            *loc,
                            format!(
                                "missing argument '{}' to function '{}'",
                                param.name_as_str(),
                                func_name.name,
                            ),
                        ));
                        continue;
                    }
                };

                let arg = match expression(
                    arg,
                    context,
                    ns,
                    symtable,
                    &mut errors,
                    ResolveTo::Type(&param.ty),
                ) {
                    Ok(e) => e,
                    Err(()) => {
                        matches = false;
                        continue;
                    }
                };

                match arg.cast(&arg.loc(), &param.ty, true, ns, &mut errors) {
                    Ok(expr) => cast_args.push(expr),
                    Err(()) => {
                        matches = false;
                        break;
                    }
                }
            }

            if matches {
                if !ns.functions[*function_no].is_public() {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "function '{}' is not 'public' or 'external'",
                            func_name.name
                        ),
                    ));
                } else if let Some(value) = &call_args.value {
                    if !value.const_zero(ns) && !ns.functions[*function_no].is_payable() {
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "sending value to function '{}' which is not payable",
                                func_name.name
                            ),
                        ));
                    }
                }

                let func = &ns.functions[*function_no];
                let returns = function_returns(func, resolve_to);
                let ty = function_type(func, true, resolve_to);

                return Ok(Expression::ExternalFunctionCall {
                    loc: *loc,
                    returns,
                    function: Box::new(Expression::ExternalFunction {
                        loc: *loc,
                        ty,
                        function_no: *function_no,
                        address: Box::new(var_expr.cast(
                            &var.loc(),
                            &Type::Contract(func.contract_no.unwrap()),
                            true,
                            ns,
                            diagnostics,
                        )?),
                    }),
                    args: cast_args,
                    call_args,
                });
            } else if name_matches.len() > 1 && diagnostics.extend_non_casting(&errors) {
                return Err(());
            }
        }

        match name_matches.len() {
            0 => {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "contract '{}' does not have function '{}'",
                        var_ty.deref_any().to_string(ns),
                        func_name.name
                    ),
                ));
            }
            1 => diagnostics.extend(errors),
            _ => {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "cannot find overloaded function which matches signature".to_string(),
                ));
            }
        }
        return Err(());
    }

    diagnostics.push(Diagnostic::error(
        func_name.loc,
        format!("method '{}' does not exist", func_name.name),
    ));

    Err(())
}

/// Function call arguments
pub fn collect_call_args<'a>(
    expr: &'a pt::Expression,
    diagnostics: &mut Diagnostics,
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
                if let Some(pt::Loc::File(file_no, start, _)) = loc {
                    loc = Some(pt::Loc::File(file_no, start, block.loc().end()));
                } else {
                    loc = Some(block.loc());
                }

                named_arguments.extend(args);
            }
            pt::Statement::Block { statements, .. } if statements.is_empty() => {
                // {}
                diagnostics.push(Diagnostic::error(
                    block.loc(),
                    "missing call arguments".to_string(),
                ));
                return Err(());
            }
            _ => {
                diagnostics.push(Diagnostic::error(
                    block.loc(),
                    "code block found where list of call arguments expected, like '{gas: 5000}'"
                        .to_string(),
                ));
                return Err(());
            }
        }

        expr = e;
    }

    Ok((expr, named_arguments, loc))
}

/// Parse call arguments for external calls
pub(super) fn parse_call_args(
    loc: &pt::Loc,
    call_args: &[&pt::NamedArgument],
    external_call: bool,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<CallArgs, ()> {
    let mut args: HashMap<&String, &pt::NamedArgument> = HashMap::new();

    for arg in call_args {
        if let Some(prev) = args.get(&arg.name.name) {
            diagnostics.push(Diagnostic::error_with_note(
                arg.loc,
                format!("'{}' specified multiple times", arg.name.name),
                prev.loc,
                format!("location of previous declaration of '{}'", arg.name.name),
            ));
            return Err(());
        }

        args.insert(&arg.name.name, arg);
    }

    let mut res = CallArgs::default();

    for arg in args.values() {
        match arg.name.name.as_str() {
            "value" => {
                if ns.target == Target::Solana {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        "Solana Cross Program Invocation (CPI) cannot transfer native value. See https://solang.readthedocs.io/en/latest/language/functions.html#value_transfer".to_string(),
                    ));

                    expression(
                        &arg.expr,
                        context,
                        ns,
                        symtable,
                        diagnostics,
                        ResolveTo::Unknown,
                    )?;
                } else {
                    let ty = Type::Value;

                    let expr = expression(
                        &arg.expr,
                        context,
                        ns,
                        symtable,
                        diagnostics,
                        ResolveTo::Type(&ty),
                    )?;

                    res.value = Some(Box::new(expr.cast(
                        &arg.expr.loc(),
                        &ty,
                        true,
                        ns,
                        diagnostics,
                    )?));
                }
            }
            "gas" => {
                if ns.target == Target::Solana {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        format!(
                            "'gas' not permitted for external calls or constructors on {}",
                            ns.target
                        ),
                    ));
                    return Err(());
                }
                let ty = Type::Uint(64);

                let expr = expression(
                    &arg.expr,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&ty),
                )?;

                res.gas = Some(Box::new(expr.cast(
                    &arg.expr.loc(),
                    &ty,
                    true,
                    ns,
                    diagnostics,
                )?));
            }
            "address" => {
                if ns.target != Target::Solana {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        format!(
                            "'address' not permitted for external calls or constructors on {}",
                            ns.target
                        ),
                    ));
                    return Err(());
                }

                if external_call {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        "'address' not valid for external calls".to_string(),
                    ));
                    return Err(());
                }

                let ty = Type::Address(false);

                let expr = expression(
                    &arg.expr,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&ty),
                )?;

                res.address = Some(Box::new(expr.cast(
                    &arg.expr.loc(),
                    &ty,
                    true,
                    ns,
                    diagnostics,
                )?));
            }
            "salt" => {
                if ns.target == Target::Solana {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        format!(
                            "'salt' not permitted for external calls or constructors on {}",
                            ns.target
                        ),
                    ));
                    return Err(());
                }

                if external_call {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        "'salt' not valid for external calls".to_string(),
                    ));
                    return Err(());
                }

                let ty = Type::Uint(256);

                let expr = expression(
                    &arg.expr,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&ty),
                )?;

                res.salt = Some(Box::new(expr.cast(
                    &arg.expr.loc(),
                    &ty,
                    true,
                    ns,
                    diagnostics,
                )?));
            }
            "accounts" => {
                if ns.target != Target::Solana {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        format!(
                            "'accounts' not permitted for external calls or constructors on {}",
                            ns.target
                        ),
                    ));
                    return Err(());
                }

                let expr = expression(
                    &arg.expr,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Unknown,
                )?;

                let mut correct_ty = false;
                let expr_ty = expr.ty();

                // if let chains would really help here
                if let Type::Array(elem_ty, dims) = expr_ty.deref_memory() {
                    if elem_ty.is_builtin_struct() == Some(StructType::AccountMeta)
                        && dims.len() == 1
                    {
                        correct_ty = true;
                    }
                }

                if !correct_ty {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        format!(
                            "'accounts' takes array of AccountMeta, not '{}'",
                            expr_ty.to_string(ns)
                        ),
                    ));
                    return Err(());
                } else if expr_ty.is_dynamic(ns) {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        "dynamic array is not supported for the 'accounts' argument".to_string(),
                    ));
                }

                res.accounts = Some(Box::new(expr));
            }
            "seeds" => {
                if ns.target != Target::Solana {
                    diagnostics.push(Diagnostic::error(
                        arg.loc,
                        format!(
                            "'seeds' not permitted for external calls or constructors on {}",
                            ns.target
                        ),
                    ));
                    return Err(());
                }

                let ty = Type::Slice(Box::new(Type::Slice(Box::new(Type::Bytes(1)))));

                let expr = expression(
                    &arg.expr,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&ty),
                )?;

                res.seeds = Some(Box::new(expr));
            }
            _ => {
                diagnostics.push(Diagnostic::error(
                    arg.loc,
                    format!("'{}' not a valid call parameter", arg.name.name),
                ));
                return Err(());
            }
        }
    }

    // address is required on solana constructors
    if ns.target == Target::Solana && !external_call {
        if res.address.is_none() && res.accounts.is_none() {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "either 'address' or 'accounts' call argument is required on {}",
                    ns.target
                ),
            ));
            return Err(());
        } else if res.address.is_some() && res.accounts.is_some() {
            diagnostics.push(Diagnostic::error(
                *loc,
                "'address' and 'accounts' call arguments cannot be used together. \
                The first address provided on the accounts vector must be the contract's address."
                    .to_string(),
            ));
            return Err(());
        }
    }

    Ok(res)
}

pub fn named_call_expr(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::NamedArgument],
    is_destructible: bool,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let mut nullsink = Diagnostics::default();

    // is it a struct literal
    match ns.resolve_type(
        context.file_no,
        context.contract_no,
        true,
        ty,
        &mut nullsink,
    ) {
        Ok(Type::Struct(str_ty)) => {
            return named_struct_literal(loc, &str_ty, args, context, ns, symtable, diagnostics);
        }
        Ok(_) => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "struct or function expected".to_string(),
            ));
            return Err(());
        }
        _ => {}
    }

    // not a struct literal, remove those errors and try resolving as function call
    if context.constant {
        diagnostics.push(Diagnostic::error(
            *loc,
            "cannot call function in constant expression".to_string(),
        ));
        return Err(());
    }

    let expr = named_function_call_expr(
        loc,
        ty,
        args,
        context,
        ns,
        symtable,
        diagnostics,
        resolve_to,
    )?;

    check_function_call(ns, &expr, symtable);
    if expr.tys().len() > 1 && !is_destructible {
        diagnostics.push(Diagnostic::error(
            *loc,
            "destucturing statement needed for function that returns multiple values".to_string(),
        ));
        return Err(());
    }

    Ok(expr)
}

/// Resolve any callable expression
pub fn call_expr(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::Expression],
    is_destructible: bool,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let mut nullsink = Diagnostics::default();
    let ty = ty.remove_parenthesis();

    match ns.resolve_type(
        context.file_no,
        context.contract_no,
        true,
        ty,
        &mut nullsink,
    ) {
        Ok(Type::Struct(str_ty)) => {
            return struct_literal(loc, &str_ty, args, context, ns, symtable, diagnostics);
        }
        Ok(to) => {
            // Cast
            return if args.is_empty() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "missing argument to cast".to_string(),
                ));
                Err(())
            } else if args.len() > 1 {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "too many arguments to cast".to_string(),
                ));
                Err(())
            } else {
                let expr = expression(
                    &args[0],
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Unknown,
                )?;

                expr.cast(loc, &to, false, ns, diagnostics)
            };
        }
        Err(_) => (),
    }

    let expr = match ty.remove_parenthesis() {
        pt::Expression::New(_, ty) => new(loc, ty, args, context, ns, symtable, diagnostics)?,
        pt::Expression::FunctionCallBlock(loc, expr, _)
            if matches!(expr.remove_parenthesis(), pt::Expression::New(..)) =>
        {
            new(loc, ty, args, context, ns, symtable, diagnostics)?
        }
        _ => {
            deprecated_constructor_arguments(ty, diagnostics)?;

            function_call_expr(
                loc,
                ty,
                args,
                context,
                ns,
                symtable,
                diagnostics,
                resolve_to,
            )?
        }
    };

    check_function_call(ns, &expr, symtable);
    if expr.tys().len() > 1 && !is_destructible {
        diagnostics.push(Diagnostic::error(
            *loc,
            "destucturing statement needed for function that returns multiple values".to_string(),
        ));
        return Err(());
    }

    Ok(expr)
}

/// Resolve function call
pub fn function_call_expr(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::Expression],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let (ty, call_args, call_args_loc) = collect_call_args(ty, diagnostics)?;

    match ty.remove_parenthesis() {
        pt::Expression::MemberAccess(_, member, func) => {
            if context.constant {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "cannot call function in constant expression".to_string(),
                ));
                return Err(());
            }

            method_call_pos_args(
                loc,
                member,
                func,
                args,
                &call_args,
                call_args_loc,
                context,
                ns,
                symtable,
                diagnostics,
                resolve_to,
            )
        }
        pt::Expression::Variable(id) => {
            // is it a builtin
            if builtin::is_builtin_call(None, &id.name, ns) {
                return {
                    let expr = builtin::resolve_call(
                        &id.loc,
                        None,
                        &id.name,
                        args,
                        context,
                        ns,
                        symtable,
                        diagnostics,
                    )?;

                    if expr.tys().len() > 1 {
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!("builtin function '{}' returns more than one value", id.name),
                        ));
                        Err(())
                    } else {
                        Ok(expr)
                    }
                };
            }

            if context.constant {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "cannot call function in constant expression".to_string(),
                ));
                return Err(());
            }

            // is there a local variable or contract variable with this name
            if symtable.find(&id.name).is_some()
                || matches!(
                    ns.resolve_var(context.file_no, context.contract_no, id, true),
                    Some(Symbol::Variable(..))
                )
            {
                call_function_type(
                    loc,
                    ty,
                    args,
                    &call_args,
                    call_args_loc,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    resolve_to,
                )
            } else {
                if let Some(loc) = call_args_loc {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        "call arguments not permitted for internal calls".to_string(),
                    ));
                    return Err(());
                }

                function_call_pos_args(
                    loc,
                    id,
                    pt::FunctionTy::Function,
                    args,
                    available_functions(&id.name, true, context.file_no, context.contract_no, ns),
                    true,
                    context,
                    ns,
                    resolve_to,
                    symtable,
                    diagnostics,
                )
            }
        }
        _ => call_function_type(
            loc,
            ty,
            args,
            &call_args,
            call_args_loc,
            context,
            ns,
            symtable,
            diagnostics,
            resolve_to,
        ),
    }
}

/// Resolve function call expression with named arguments
pub fn named_function_call_expr(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::NamedArgument],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    resolve_to: ResolveTo,
) -> Result<Expression, ()> {
    let (ty, call_args, call_args_loc) = collect_call_args(ty, diagnostics)?;

    match ty {
        pt::Expression::MemberAccess(_, member, func) => method_call_named_args(
            loc,
            member,
            func,
            args,
            &call_args,
            call_args_loc,
            context,
            ns,
            symtable,
            diagnostics,
            resolve_to,
        ),
        pt::Expression::Variable(id) => {
            if let Some(loc) = call_args_loc {
                diagnostics.push(Diagnostic::error(
                    loc,
                    "call arguments not permitted for internal calls".to_string(),
                ));
                return Err(());
            }

            function_call_named_args(
                loc,
                id,
                args,
                available_functions(&id.name, true, context.file_no, context.contract_no, ns),
                true,
                context,
                resolve_to,
                ns,
                symtable,
                diagnostics,
            )
        }
        pt::Expression::ArraySubscript(..) => {
            diagnostics.push(Diagnostic::error(
                ty.loc(),
                "unexpected array type".to_string(),
            ));
            Err(())
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                ty.loc(),
                "expression not expected here".to_string(),
            ));
            Err(())
        }
    }
}

/// Get the return values for a function call
pub(crate) fn function_returns(ftype: &Function, resolve_to: ResolveTo) -> Vec<Type> {
    if !ftype.returns.is_empty() && !matches!(resolve_to, ResolveTo::Discard) {
        ftype.returns.iter().map(|p| p.ty.clone()).collect()
    } else {
        vec![Type::Void]
    }
}

/// Get the function type for an internal.external function call
pub(crate) fn function_type(func: &Function, external: bool, resolve_to: ResolveTo) -> Type {
    let params = func.params.iter().map(|p| p.ty.clone()).collect();
    let mutability = func.mutability.clone();
    let returns = function_returns(func, resolve_to);

    if external {
        Type::ExternalFunction {
            params,
            mutability,
            returns,
        }
    } else {
        Type::InternalFunction {
            params,
            mutability,
            returns,
        }
    }
}

/// This function evaluates the arguments of a function call with either positional arguments or
/// named arguments.
fn evaluate_argument(
    arg: &pt::Expression,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    arg_ty: &Type,
    errors: &mut Diagnostics,
    cast_args: &mut Vec<Expression>,
) -> bool {
    let arg = match expression(arg, context, ns, symtable, errors, ResolveTo::Type(arg_ty)) {
        Ok(e) => e,
        Err(_) => {
            return false;
        }
    };

    match arg.cast(&arg.loc(), arg_ty, true, ns, errors) {
        Ok(expr) => cast_args.push(expr),
        Err(_) => return false,
    }

    true
}

/// This function finishes resolving internal function calls
fn resolve_internal_call(
    loc: &Loc,
    function_no: usize,
    context: &ExprContext,
    resolve_to: ResolveTo,
    virtual_call: bool,
    cast_args: Vec<Expression>,
    ns: &Namespace,
    errors: &mut Diagnostics,
) -> Option<Expression> {
    let func = &ns.functions[function_no];

    if func.contract_no != context.contract_no && func.is_private() {
        errors.push(Diagnostic::error_with_note(
            *loc,
            format!("cannot call private {}", func.ty),
            func.loc,
            format!("declaration of {} '{}'", func.ty, func.name),
        ));

        return None;
    } else if func.contract_no == context.contract_no
        && matches!(func.visibility, Visibility::External(_))
    {
        errors.push(Diagnostic::error(
            *loc,
            "external functions can only be invoked outside the contract".to_string(),
        ));
        return None;
    }

    let returns = function_returns(func, resolve_to);
    let ty = function_type(func, false, resolve_to);

    Some(Expression::InternalFunctionCall {
        loc: *loc,
        returns,
        function: Box::new(Expression::InternalFunction {
            loc: *loc,
            ty,
            function_no,
            signature: if virtual_call && (func.is_virtual || func.is_override.is_some()) {
                Some(func.signature.clone())
            } else {
                None
            },
        }),
        args: cast_args,
    })
}
