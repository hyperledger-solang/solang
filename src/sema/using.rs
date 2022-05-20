use super::{
    ast::{Diagnostic, Expression, Namespace, Note, Type, Using, UsingList},
    expression::{expression, function_returns, function_type, ExprContext, ResolveTo},
    symtable::Symtable,
};
use crate::parser::pt;
use crate::parser::pt::CodeLocation;
use std::collections::HashSet;

/// Resolve a using declaration in either file scope or contract scope
pub(crate) fn using_decl(
    using: &pt::Using,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
) -> Result<Using, ()> {
    let mut diagnostics = Vec::new();

    let ty = if let Some(expr) = &using.ty {
        match ns.resolve_type(file_no, contract_no, false, expr, &mut diagnostics) {
            Ok(Type::Contract(contract_no)) if ns.contracts[contract_no].is_library() => {
                ns.diagnostics.push(Diagnostic::error(
                    expr.loc(),
                    format!("using for library '{}' type not permitted", expr),
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

            for function_name in functions {
                if let Ok(list) = ns.resolve_free_function_with_namespace(
                    file_no,
                    function_name,
                    &mut diagnostics,
                ) {
                    if list.len() > 1 {
                        let notes = list
                            .iter()
                            .map(|(loc, _)| Note {
                                loc: *loc,
                                message: format!("definition of '{}'", function_name),
                            })
                            .collect();

                        diagnostics.push(Diagnostic::error_with_notes(
                            function_name.loc,
                            format!("'{}' is an overloaded function", function_name),
                            notes,
                        ));
                        continue;
                    }

                    let (loc, func_no) = list[0];

                    let func = &ns.functions[func_no];

                    if func.params.is_empty() {
                        diagnostics.push(Diagnostic::error_with_note(
                            function_name.loc,
                            format!(
                                "'{}' has no arguments, at least one argument required",
                                function_name
                            ),
                            loc,
                            format!("definition of '{}'", function_name),
                        ));
                        continue;
                    }

                    if let Some(ty) = &ty {
                        if *ty != func.params[0].ty {
                            diagnostics.push(Diagnostic::error_with_note(
                                function_name.loc,
                                format!("function cannot be used since first argument is '{}' rather than the required '{}'", func.params[0].ty.to_string(ns), ty.to_string(ns)),
                                loc,
                                format!("definition of '{}'", function_name),
                            ));
                            continue;
                        }
                    }

                    res.push(func_no);
                }
            }

            UsingList::Functions(res)
        }
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
    let mut diagnostics = Vec::new();
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
        .flat_map(|using| match &using.list {
            UsingList::Library(library_no) => ns.contracts[*library_no].functions.iter(),
            UsingList::Functions(functions) => functions.iter(),
        })
        .filter(|func_no| {
            let func = &ns.functions[**func_no];

            func.name == function_name && func.ty == pt::FunctionTy::Function
        })
        .cloned()
        .collect()
}

pub(super) fn try_resolve_using_call(
    loc: &pt::Loc,
    func: &pt::Identifier,
    self_expr: &Expression,
    context: &ExprContext,
    args: &[pt::Expression],
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
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
    let mut errors = Vec::new();

    for function_no in functions {
        let libfunc = &ns.functions[function_no];
        if libfunc.name != func.name || libfunc.ty != pt::FunctionTy::Function {
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
                libfunc.loc,
                format!("declaration of function '{}'", libfunc.name),
            ));

            continue;
        }

        let returns = function_returns(libfunc, resolve_to);
        let ty = function_type(libfunc, false, resolve_to);

        return Ok(Some(Expression::InternalFunctionCall {
            loc: *loc,
            returns,
            function: Box::new(Expression::InternalFunction {
                loc: *loc,
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
