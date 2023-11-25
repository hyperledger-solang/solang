// SPDX-License-Identifier: Apache-2.0

use super::{
    ast::{
        Diagnostic, Expression, Mutability, Namespace, Note, Type, Using, UsingFunction, UsingList,
    },
    diagnostics::Diagnostics,
    expression::{ExprContext, ResolveTo},
    symtable::Symtable,
};
use crate::sema::expression::function_call::{function_returns, function_type};
use crate::sema::expression::resolve_expression::expression;
use crate::sema::namespace::ResolveTypeContext;
use solang_parser::pt::CodeLocation;
use solang_parser::pt::{self};
use std::collections::HashSet;

/// Resolve a using declaration in either file scope or contract scope
pub(crate) fn using_decl(
    using: &pt::Using,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
) -> Result<Using, ()> {
    let mut diagnostics = Diagnostics::default();

    if let Some(contract_no) = contract_no {
        if ns.contracts[contract_no].is_interface() {
            ns.diagnostics.push(Diagnostic::error(
                using.loc,
                "using for not permitted in interface".into(),
            ));
            return Err(());
        }
    }

    let ty = if let Some(expr) = &using.ty {
        match ns.resolve_type(
            file_no,
            contract_no,
            ResolveTypeContext::None,
            expr,
            &mut diagnostics,
        ) {
            Ok(Type::Contract(contract_no)) if ns.contracts[contract_no].is_library() => {
                ns.diagnostics.push(Diagnostic::error(
                    expr.loc(),
                    format!("using for library '{expr}' type not permitted"),
                ));
                return Err(());
            }
            Ok(ty) => Some(ty),
            Err(_) => {
                ns.diagnostics.extend(diagnostics);
                return Err(());
            }
        }
    } else {
        if contract_no.is_none() {
            ns.diagnostics.push(Diagnostic::error(
                using.loc,
                "using must be bound to specific type, '*' cannot be used on file scope"
                    .to_string(),
            ));
            return Err(());
        }
        None
    };

    let list = match &using.list {
        pt::UsingList::Library(library) => {
            if let Ok(library_no) =
                ns.resolve_contract_with_namespace(file_no, library, &mut diagnostics)
            {
                if ns.contracts[library_no].is_library() {
                    UsingList::Library(library_no)
                } else {
                    ns.diagnostics.push(Diagnostic::error(
                        library.loc,
                        format!(
                            "library expected but {} '{}' found",
                            ns.contracts[library_no].ty, library
                        ),
                    ));
                    return Err(());
                }
            } else {
                ns.diagnostics.extend(diagnostics);
                return Err(());
            }
        }
        pt::UsingList::Functions(functions) => {
            let mut res = Vec::new();

            for using_function in functions {
                let function_name = &using_function.path;
                if let Ok(list) = ns.resolve_function_with_namespace(
                    file_no,
                    contract_no,
                    &using_function.path,
                    &mut diagnostics,
                ) {
                    if list.len() > 1 {
                        let notes = list
                            .iter()
                            .map(|(loc, _)| Note {
                                loc: *loc,
                                message: format!("definition of '{function_name}'"),
                            })
                            .collect();

                        diagnostics.push(Diagnostic::error_with_notes(
                            function_name.loc,
                            format!("'{function_name}' is an overloaded function"),
                            notes,
                        ));
                        continue;
                    }

                    let (loc, func_no) = list[0];

                    let func = &ns.functions[func_no];

                    if let Some(contract_no) = func.contract_no {
                        if !ns.contracts[contract_no].is_library() {
                            diagnostics.push(Diagnostic::error_with_note(
                                function_name.loc,
                                format!("'{function_name}' is not a library function"),
                                func.loc_prototype,
                                format!("definition of {}", using_function.path),
                            ));
                            continue;
                        }
                    }

                    if func.params.is_empty() {
                        diagnostics.push(Diagnostic::error_with_note(
                            function_name.loc,
                            format!(
                                "'{function_name}' has no arguments. At least one argument required"

                            ),
                            loc,
                            format!("definition of '{function_name}'"),
                        ));
                        continue;
                    }

                    let oper = if let Some(mut oper) = using_function.oper {
                        if contract_no.is_some() || using.global.is_none() || ty.is_none() {
                            diagnostics.push(Diagnostic::error(
                                using_function.loc,
                                "user defined operator can only be set in a global 'using for' directive".into(),
                            ));
                            break;
                        }

                        let ty = ty.as_ref().unwrap();

                        if !matches!(*ty, Type::UserType(_)) {
                            diagnostics.push(Diagnostic::error(
                                using_function.loc,
                                format!("user defined operator can only be used with user defined types. Type {} not permitted", ty.to_string(ns))
                            ));
                            break;
                        }

                        // The '-' operator may be for subtract or negation, the parser cannot know which one it was
                        if oper == pt::UserDefinedOperator::Subtract
                            || oper == pt::UserDefinedOperator::Negate
                        {
                            oper = match func.params.len() {
                                1 => pt::UserDefinedOperator::Negate,
                                2 => pt::UserDefinedOperator::Subtract,
                                _ => {
                                    diagnostics.push(Diagnostic::error_with_note(
                                        using_function.loc,
                                            "user defined operator function for '-' must have 1 parameter for negate, or 2 parameters for subtract".into(),
                                        loc,
                                        format!("definition of '{function_name}'"),
                                    ));
                                    continue;
                                }
                            }
                        };

                        if func.params.len() != oper.args()
                            || func.params.iter().any(|param| param.ty != *ty)
                        {
                            diagnostics.push(Diagnostic::error_with_note(
                                using_function.loc,
                                format!(
                                    "user defined operator function for '{}' must have {} arguments of type {}",
                                    oper, oper.args(), ty.to_string(ns)
                                ),
                                loc,
                                format!("definition of '{function_name}'"),
                            ));
                            continue;
                        }

                        if oper.is_comparison() {
                            if func.returns.len() != 1 || func.returns[0].ty != Type::Bool {
                                diagnostics.push(Diagnostic::error_with_note(
                                    using_function.loc,
                                    format!(
                                        "user defined operator function for '{oper}' must have one bool return type",
                                    ),
                                    loc,
                                    format!("definition of '{function_name}'"),
                                ));
                                continue;
                            }
                        } else if func.returns.len() != 1 || func.returns[0].ty != *ty {
                            diagnostics.push(Diagnostic::error_with_note(
                                using_function.loc,
                                    format!(
                                        "user defined operator function for '{}' must have single return type {}",
                                        oper, ty.to_string(ns)
                                    ),
                                    loc,
                                    format!("definition of '{function_name}'"),
                                ));
                            continue;
                        }

                        if !matches!(func.mutability, Mutability::Pure(_)) {
                            diagnostics.push(Diagnostic::error_with_note(
                                using_function.loc,
                                format!(
                                    "user defined operator function for '{oper}' must have pure mutability",
                                ),
                                loc,
                                format!("definition of '{function_name}'"),
                            ));
                            continue;
                        }

                        if let Some(existing) = user_defined_operator_binding(ty, oper, ns) {
                            if existing.function_no != func_no {
                                diagnostics.push(Diagnostic::error_with_note(
                                    using_function.loc,
                                    format!("user defined operator for '{oper}' redefined"),
                                    existing.loc,
                                    format!(
                                        "previous definition of '{oper}' was '{}'",
                                        ns.functions[existing.function_no].id
                                    ),
                                ));
                            } else {
                                diagnostics.push(Diagnostic::warning_with_note(
                                    using_function.loc,
                                    format!("user defined operator for '{oper}' redefined to same function"),
                                    existing.loc,
                                    format!(
                                        "previous definition of '{oper}' was '{}'",
                                        ns.functions[existing.function_no].id
                                    ),
                                ));
                            }
                            continue;
                        }

                        Some(oper)
                    } else {
                        if let Some(ty) = &ty {
                            let dummy = Expression::Variable {
                                loc,
                                ty: ty.clone(),
                                var_no: 0,
                            };

                            if dummy
                                .cast(
                                    &loc,
                                    &func.params[0].ty,
                                    true,
                                    ns,
                                    &mut Diagnostics::default(),
                                )
                                .is_err()
                            {
                                diagnostics.push(Diagnostic::error_with_note(
                                    function_name.loc,
                                    format!("function cannot be used since first argument is '{}' rather than the required '{}'", func.params[0].ty.to_string(ns), ty.to_string(ns)),
                                    loc,
                                    format!("definition of '{function_name}'"),
                                ));
                                continue;
                            }
                        }

                        None
                    };

                    res.push(UsingFunction {
                        loc: using_function.loc,
                        function_no: func_no,
                        oper,
                    });
                }
            }

            UsingList::Functions(res)
        }

        pt::UsingList::Error => unimplemented!(),
    };

    let mut file_no = Some(file_no);

    if let Some(global) = &using.global {
        if global.name == "global" {
            if contract_no.is_some() {
                ns.diagnostics.push(Diagnostic::error(
                    global.loc,
                    format!("'{}' on using within contract not permitted", global.name),
                ));
            } else {
                match &ty {
                    Some(Type::Struct(_)) | Some(Type::UserType(_)) | Some(Type::Enum(_)) => {
                        file_no = None;
                    }
                    _ => {
                        ns.diagnostics.push(Diagnostic::error(
                            global.loc,
                            format!("'{}' only permitted on user defined types", global.name),
                        ));
                    }
                }
            }
        } else {
            ns.diagnostics.push(Diagnostic::error(
                global.loc,
                format!("'{}' not expected, did you mean 'global'?", global.name),
            ));
        }
    }

    ns.diagnostics.extend(diagnostics);

    Ok(Using { list, ty, file_no })
}

/// Given the using declarations, find all the possible functions that could be called via using
/// for the given name, type and file scope
fn possible_functions(
    using: &[Using],
    file_no: usize,
    function_name: &str,
    self_expr: &Expression,
    ns: &Namespace,
) -> HashSet<usize> {
    let mut diagnostics = Diagnostics::default();
    using
        .iter()
        .filter(|using| {
            if let Some(ty) = &using.ty {
                self_expr
                    .cast(&self_expr.loc(), ty, true, ns, &mut diagnostics)
                    .is_ok()
            } else {
                true
            }
        })
        .filter(|using| {
            if let Some(no) = using.file_no {
                no == file_no
            } else {
                true
            }
        })
        .flat_map(|using| {
            let iterator: Box<dyn Iterator<Item = _>> = match &using.list {
                UsingList::Library(library_no) => {
                    Box::new(ns.contracts[*library_no].functions.iter())
                }
                UsingList::Functions(functions) => {
                    Box::new(functions.iter().filter_map(move |using| {
                        if using.oper.is_none() {
                            Some(&using.function_no)
                        } else {
                            None
                        }
                    }))
                }
            };

            iterator
        })
        .filter(|func_no| {
            let func = &ns.functions[**func_no];

            func.id.name == function_name && func.ty == pt::FunctionTy::Function
        })
        .cloned()
        .collect()
}

/// Given the type and oper, find the user defined operator function binding. Note there can only be one.
pub(crate) fn user_defined_operator_binding<'a>(
    ty: &Type,
    oper: pt::UserDefinedOperator,
    ns: &'a Namespace,
) -> Option<&'a UsingFunction> {
    let oper = Some(oper);

    ns.using
        .iter()
        .filter(|using| Some(ty) == using.ty.as_ref())
        .find_map(|using| {
            if let UsingList::Functions(funcs) = &using.list {
                funcs.iter().find(|using| using.oper == oper)
            } else {
                None
            }
        })
}

pub(super) fn try_resolve_using_call(
    loc: &pt::Loc,
    func: &pt::Identifier,
    self_expr: &Expression,
    context: &mut ExprContext,
    args: &[pt::Expression],
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
    ns: &mut Namespace,
    resolve_to: ResolveTo,
) -> Result<Option<Expression>, ()> {
    // first collect all possible functions that could be used for using
    // Use HashSet for deduplication.
    // If the using directive specifies a type, the type must match the type of
    // the method call object exactly.
    let mut functions = possible_functions(&ns.using, context.file_no, &func.name, self_expr, ns);

    if let Some(contract_no) = context.contract_no {
        functions.extend(possible_functions(
            &ns.contracts[contract_no].using,
            context.file_no,
            &func.name,
            self_expr,
            ns,
        ));
    }

    let mut name_matches = 0;
    let mut errors = Diagnostics::default();

    for function_no in functions {
        let libfunc = &ns.functions[function_no];
        if libfunc.id.name != func.name || libfunc.ty != pt::FunctionTy::Function {
            continue;
        }

        name_matches += 1;

        let params_len = libfunc.params.len();

        if params_len != args.len() + 1 {
            errors.push(Diagnostic::error(
                *loc,
                format!(
                    "using function expects {} arguments, {} provided (including self)",
                    params_len,
                    args.len() + 1
                ),
            ));
            continue;
        }
        let mut matches = true;
        let mut cast_args = Vec::new();

        match self_expr.cast(
            &self_expr.loc(),
            &libfunc.params[0].ty,
            true,
            ns,
            &mut errors,
        ) {
            Ok(e) => cast_args.push(e),
            Err(()) => continue,
        }

        // check if arguments can be implicitly casted
        for (i, arg) in args.iter().enumerate() {
            let ty = ns.functions[function_no].params[i + 1].ty.clone();

            let arg = match expression(
                arg,
                context,
                ns,
                symtable,
                &mut errors,
                ResolveTo::Type(&ty),
            ) {
                Ok(e) => e,
                Err(()) => {
                    matches = false;
                    continue;
                }
            };

            match arg.cast(&arg.loc(), &ty, true, ns, &mut errors) {
                Ok(expr) => cast_args.push(expr),
                Err(_) => {
                    matches = false;
                    break;
                }
            }
        }
        if !matches {
            continue;
        }

        let libfunc = &ns.functions[function_no];

        if libfunc.is_private() {
            errors.push(Diagnostic::error_with_note(
                *loc,
                "cannot call private library function".to_string(),
                libfunc.loc_prototype,
                format!("declaration of function '{}'", libfunc.id),
            ));

            continue;
        }

        let returns = function_returns(libfunc, resolve_to);
        let ty = function_type(libfunc, false, resolve_to);

        let id_path = pt::IdentifierPath {
            loc: func.loc,
            identifiers: vec![func.clone()],
        };

        return Ok(Some(Expression::InternalFunctionCall {
            loc: *loc,
            returns,
            function: Box::new(Expression::InternalFunction {
                loc: *loc,
                id: id_path,
                ty,
                function_no,
                signature: None,
            }),
            args: cast_args,
        }));
    }

    match name_matches {
        0 => Ok(None),
        1 => {
            diagnostics.extend(errors);

            Err(())
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "cannot find overloaded function which matches signature".to_string(),
            ));
            Err(())
        }
    }
}
