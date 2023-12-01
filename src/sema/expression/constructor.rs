// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{ArrayLength, CallArgs, Expression, Namespace, Note, RetrieveType, Type};
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::function_call::{
    collect_call_args, evaluate_argument, parse_call_args,
};
use crate::sema::expression::resolve_expression::expression;
use crate::sema::expression::{ExprContext, ResolveTo};
use crate::sema::namespace::ResolveTypeContext;
use crate::sema::symtable::Symtable;
use crate::sema::unused_variable::used_variable;
use solang_parser::diagnostics::Diagnostic;
use solang_parser::pt;
use solang_parser::pt::{CodeLocation, Visibility};
use std::collections::BTreeMap;
use std::mem::swap;

/// Resolve an new contract expression with positional arguments
fn constructor(
    loc: &pt::Loc,
    no: usize,
    args: &[pt::Expression],
    call_args: CallArgs,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    if !ns.contracts[no].instantiable {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "cannot construct '{}' of type '{}'",
                ns.contracts[no].id, ns.contracts[no].ty
            ),
        ));

        return Err(());
    }

    // The current contract cannot be constructed with new. In order to create
    // the contract, we need the code hash of the contract. Part of that code
    // will be code we're emitted here. So we end up with a crypto puzzle.
    if let Some(context_contract_no) = context.contract_no {
        if context_contract_no == no {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "new cannot construct current contract '{}'",
                    ns.contracts[no].id
                ),
            ));
            return Err(());
        }

        // check for circular references
        if circular_reference(no, context_contract_no, ns) {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "circular reference creating contract '{}'",
                    ns.contracts[no].id
                ),
            ));
            return Err(());
        }

        if !ns.contracts[context_contract_no].creates.contains(&no) {
            ns.contracts[context_contract_no].creates.push(no);
        }
    }

    // This is not always in a function: e.g. contract variable:
    // contract C {
    //      D code = new D();
    // }
    if let Some(function_no) = context.function_no {
        ns.functions[function_no].creates.push((*loc, no));
    }

    match match_constructor_to_args(loc, args, no, context, ns, symtable, diagnostics) {
        Ok((constructor_no, cast_args)) => Ok(Expression::Constructor {
            loc: *loc,
            contract_no: no,
            constructor_no,
            args: cast_args,
            call_args,
        }),
        Err(()) => Err(()),
    }
}

/// Try and find constructor for arguments
pub fn match_constructor_to_args(
    loc: &pt::Loc,
    args: &[pt::Expression],
    contract_no: usize,
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<(Option<usize>, Vec<Expression>), ()> {
    // constructor call
    let function_nos: Vec<usize> = ns.contracts[contract_no]
        .functions
        .iter()
        .filter(|function_no| ns.functions[**function_no].is_constructor())
        .copied()
        .collect();

    // try to resolve the arguments, give up if there are any errors
    if args.iter().fold(false, |acc, arg| {
        acc | expression(arg, context, ns, symtable, diagnostics, ResolveTo::Unknown).is_err()
    }) {
        return Err(());
    }

    let mut call_diagnostics = Diagnostics::default();
    let mut resolved_calls = Vec::new();

    for function_no in &function_nos {
        let params_len = ns.functions[*function_no].params.len();
        let mut candidate_diagnostics = Diagnostics::default();
        let mut cast_args = Vec::new();

        if params_len != args.len() {
            candidate_diagnostics.push(Diagnostic::cast_error(
                *loc,
                format!(
                    "constructor expects {} arguments, {} provided",
                    params_len,
                    args.len()
                ),
            ));
        } else {
            // resolve arguments for this constructor
            for (i, arg) in args.iter().enumerate() {
                let ty = ns.functions[*function_no].params[i].ty.clone();

                evaluate_argument(
                    arg,
                    context,
                    ns,
                    symtable,
                    &ty,
                    &mut candidate_diagnostics,
                    &mut cast_args,
                );
            }
        }

        if candidate_diagnostics.any_errors() {
            if function_nos.len() != 1 {
                let func = &ns.functions[*function_no];

                candidate_diagnostics.iter_mut().for_each(|diagnostic| {
                    diagnostic.notes.push(Note {
                        loc: func.loc,
                        message: "candidate constructor".into(),
                    })
                });

                // will be de-duped
                candidate_diagnostics.push(Diagnostic::error(
                    *loc,
                    "cannot find overloaded constructor which matches signature".into(),
                ));
            }
        } else {
            resolved_calls.push((Some(*function_no), cast_args));
            continue;
        }

        call_diagnostics.extend(candidate_diagnostics);
    }

    match resolved_calls.len() {
        0 if function_nos.is_empty() => {
            if args.is_empty() {
                Ok((None, Vec::new()))
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "default constructor does not take arguments".into(),
                ));
                Err(())
            }
        }
        0 => {
            diagnostics.extend(call_diagnostics);

            Err(())
        }
        1 => Ok(resolved_calls.remove(0)),
        _ => {
            diagnostics.push(Diagnostic::error_with_notes(
                *loc,
                "constructor can be resolved to multiple functions".into(),
                resolved_calls
                    .iter()
                    .map(|(func_no, _)| {
                        let func = &ns.functions[func_no.unwrap()];

                        Note {
                            loc: func.loc,
                            message: "candidate constructor".into(),
                        }
                    })
                    .collect(),
            ));
            Err(())
        }
    }
}

/// check if from creates to, recursively
pub(super) fn circular_reference(from: usize, to: usize, ns: &Namespace) -> bool {
    if ns.contracts[from].creates.contains(&to) {
        return true;
    }

    ns.contracts[from]
        .creates
        .iter()
        .any(|n| circular_reference(*n, to, ns))
}

/// Resolve an new contract expression with named arguments
pub fn constructor_named_args(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::NamedArgument],
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let (ty, call_args, _) = collect_call_args(ty, diagnostics)?;

    let no = match ns.resolve_type(
        context.file_no,
        context.contract_no,
        ResolveTypeContext::None,
        ty,
        diagnostics,
    )? {
        Type::Contract(n) => n,
        _ => {
            diagnostics.push(Diagnostic::error(*loc, "contract expected".to_string()));
            return Err(());
        }
    };

    let call_args = parse_call_args(
        loc,
        &call_args,
        Some(no),
        false,
        context,
        ns,
        symtable,
        diagnostics,
    )?;

    if !ns.contracts[no].instantiable {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "cannot construct '{}' of type '{}'",
                ns.contracts[no].id, ns.contracts[no].ty
            ),
        ));

        return Err(());
    }

    // The current contract cannot be constructed with new. In order to create
    // the contract, we need the code hash of the contract. Part of that code
    // will be code we're emitted here. So we end up with a crypto puzzle.

    if let Some(context_contract_no) = context.contract_no {
        if context_contract_no == no {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "new cannot construct current contract '{}'",
                    ns.contracts[no].id
                ),
            ));
            return Err(());
        }

        // check for circular references
        if circular_reference(no, context_contract_no, ns) {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "circular reference creating contract '{}'",
                    ns.contracts[no].id
                ),
            ));
            return Err(());
        }

        if !ns.contracts[context_contract_no].creates.contains(&no) {
            ns.contracts[context_contract_no].creates.push(no);
        }
    }

    // This is not always in a function: e.g. contract variable:
    // contract C {
    //      D code = new D({});
    // }
    if let Some(function_no) = context.function_no {
        ns.functions[function_no].creates.push((*loc, no));
    }

    let mut arguments: BTreeMap<&str, &pt::Expression> = BTreeMap::new();

    if args.iter().fold(false, |mut acc, arg| {
        if let Some(prev) = arguments.get(arg.name.name.as_str()) {
            diagnostics.push(Diagnostic::error_with_note(
                arg.name.loc,
                format!("duplicate argument with name '{}'", arg.name.name),
                prev.loc(),
                "location of previous argument".into(),
            ));

            let _ = expression(
                &arg.expr,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Unknown,
            );
            acc = true;
        } else {
            acc |= expression(
                &arg.expr,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Unknown,
            )
            .is_err()
        }

        arguments.insert(arg.name.name.as_str(), &arg.expr);

        acc
    }) {
        return Err(());
    }

    // constructor call
    let function_nos: Vec<usize> = ns.contracts[no]
        .functions
        .iter()
        .filter(|function_no| ns.functions[**function_no].is_constructor())
        .copied()
        .collect();

    let mut call_diagnostics = Diagnostics::default();
    let mut resolved_calls = Vec::new();

    // constructor call
    for function_no in &function_nos {
        let func = &ns.functions[*function_no];
        let params_len = func.params.len();

        let mut candidate_diagnostics = Diagnostics::default();

        let unnamed_params = func.params.iter().filter(|p| p.id.is_none()).count();
        let func_loc = ns.functions[*function_no].loc_prototype;

        let mut cast_args = Vec::new();

        if unnamed_params > 0 {
            candidate_diagnostics.push(Diagnostic::cast_error_with_note(
                *loc,
                format!(
                    "constructor cannot be called with named arguments as {unnamed_params} of its parameters do not have names"
                ),
                func.loc_prototype,
                format!("definition of {}", func.ty),
            ));
        } else if params_len != args.len() {
            candidate_diagnostics.push(Diagnostic::cast_error_with_note(
                *loc,
                format!(
                    "constructor expects {} arguments, {} provided",
                    params_len,
                    args.len()
                ),
                func.loc_prototype,
                "definition of constructor".to_owned(),
            ));
        } else {
            // check if arguments can be implicitly casted
            for i in 0..params_len {
                let param = ns.functions[*function_no].params[i].clone();

                let arg = match arguments.get(param.name_as_str()) {
                    Some(a) => a,
                    None => {
                        candidate_diagnostics.push(Diagnostic::cast_error_with_note(
                            *loc,
                            format!("missing argument '{}' to constructor", param.name_as_str()),
                            func_loc,
                            "definition of constructor".to_owned(),
                        ));
                        continue;
                    }
                };

                evaluate_argument(
                    arg,
                    context,
                    ns,
                    symtable,
                    &param.ty,
                    &mut candidate_diagnostics,
                    &mut cast_args,
                );
            }
        }

        if candidate_diagnostics.any_errors() {
            if function_nos.len() != 1 {
                let func = &ns.functions[*function_no];

                candidate_diagnostics.iter_mut().for_each(|diagnostic| {
                    diagnostic.notes.push(Note {
                        loc: func.loc,
                        message: "candidate constructor".into(),
                    })
                });

                // will be de-duped
                candidate_diagnostics.push(Diagnostic::error(
                    *loc,
                    "cannot find overloaded constructor which matches signature".into(),
                ));
            }
        } else {
            resolved_calls.push(Expression::Constructor {
                loc: *loc,
                contract_no: no,
                constructor_no: Some(*function_no),
                args: cast_args,
                call_args: call_args.clone(),
            });
            continue;
        }

        call_diagnostics.extend(candidate_diagnostics);
    }

    match resolved_calls.len() {
        0 if function_nos.is_empty() && args.is_empty() => Ok(Expression::Constructor {
            loc: *loc,
            contract_no: no,
            constructor_no: None,
            args: Vec::new(),
            call_args,
        }),
        0 => {
            diagnostics.extend(call_diagnostics);

            if function_nos.is_empty() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "cannot find matching constructor".into(),
                ));
            }

            Err(())
        }
        1 => Ok(resolved_calls.remove(0)),
        _ => {
            diagnostics.push(Diagnostic::error_with_notes(
                *loc,
                "can be resolved to multiple constructors".into(),
                resolved_calls
                    .iter()
                    .map(|expr| {
                        let Expression::Constructor { constructor_no, .. } = expr else {
                            unreachable!()
                        };
                        let func = &ns.functions[constructor_no.unwrap()];

                        Note {
                            loc: func.loc,
                            message: "candidate constructor".into(),
                        }
                    })
                    .collect(),
            ));
            Err(())
        }
    }
}

/// Resolve an new expression
pub fn new(
    loc: &pt::Loc,
    ty: &pt::Expression,
    args: &[pt::Expression],
    context: &mut ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let (ty, call_args, call_args_loc) = collect_call_args(ty, diagnostics)?;

    let ty = if let pt::Expression::New(_, ty) = ty.remove_parenthesis() {
        ty
    } else {
        ty
    };

    let ty = ns.resolve_type(
        context.file_no,
        context.contract_no,
        ResolveTypeContext::None,
        ty,
        diagnostics,
    )?;

    match &ty {
        Type::Array(ty, dim) => {
            if matches!(dim.last(), Some(ArrayLength::Fixed(_))) {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "new cannot allocate fixed array type '{}'",
                        ty.to_string(ns)
                    ),
                ));
                return Err(());
            }

            if let Type::Contract(_) = ty.as_ref() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!("new cannot construct array of '{}'", ty.to_string(ns)),
                ));
                return Err(());
            }
        }
        Type::String | Type::DynamicBytes => {}
        Type::Contract(n) => {
            let call_args = parse_call_args(
                loc,
                &call_args,
                Some(*n),
                false,
                context,
                ns,
                symtable,
                diagnostics,
            )?;

            return constructor(loc, *n, args, call_args, context, ns, symtable, diagnostics);
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!("new cannot allocate type '{}'", ty.to_string(ns)),
            ));
            return Err(());
        }
    };

    if let Some(loc) = call_args_loc {
        diagnostics.push(Diagnostic::error(
            loc,
            "constructor arguments not permitted for allocation".to_string(),
        ));
        return Err(());
    }

    if args.len() != 1 {
        diagnostics.push(Diagnostic::error(
            *loc,
            "new dynamic array should have a single length argument".to_string(),
        ));
        return Err(());
    }

    let size_loc = args[0].loc();
    let expected_ty = Type::Uint(32);

    let size_expr = expression(
        &args[0],
        context,
        ns,
        symtable,
        diagnostics,
        ResolveTo::Type(&expected_ty),
    )?;

    used_variable(ns, &size_expr, symtable);

    let size_ty = size_expr.ty();

    if !matches!(size_ty.deref_any(), Type::Uint(_)) {
        diagnostics.push(Diagnostic::error(
            size_expr.loc(),
            "new dynamic array should have an unsigned length argument".to_string(),
        ));
        return Err(());
    }

    let size = if size_ty.deref_any().bits(ns) > 32 {
        diagnostics.push(Diagnostic::warning(
            size_expr.loc(),
            format!(
                "conversion truncates {} to {}, as memory size is type {} on target {}",
                size_ty.deref_any().to_string(ns),
                expected_ty.to_string(ns),
                expected_ty.to_string(ns),
                ns.target
            ),
        ));

        Expression::CheckingTrunc {
            loc: size_loc,
            to: expected_ty.clone(),
            expr: Box::new(size_expr.cast(&size_loc, &size_ty, true, ns, diagnostics)?),
        }
    } else {
        size_expr.cast(&size_loc, &expected_ty, true, ns, diagnostics)?
    };

    Ok(Expression::AllocDynamicBytes {
        loc: *loc,
        ty,
        length: Box::new(size),
        init: None,
    })
}

/// Is it an (new C).value(1).gas(2)(1, 2, 3) style constructor (not supported)?
pub(super) fn deprecated_constructor_arguments(
    expr: &pt::Expression,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match expr.remove_parenthesis() {
        pt::Expression::FunctionCall(func_loc, ty, _) => {
            if let pt::Expression::MemberAccess(_, ty, call_arg) = ty.as_ref() {
                if deprecated_constructor_arguments(ty, diagnostics).is_err() {
                    // location should be the identifier and the arguments
                    let mut loc = call_arg.loc;
                    if let pt::Loc::File(_, _, end) = &mut loc {
                        *end = func_loc.end();
                    }
                    diagnostics.push(Diagnostic::error(
                        loc,
                        format!("deprecated call argument syntax '.{}(...)' is not supported, use '{{{}: ...}}' instead", call_arg.name, call_arg.name)
                    ));
                    return Err(());
                }
            }
        }
        pt::Expression::New(..) => {
            return Err(());
        }
        _ => (),
    }

    Ok(())
}

/// Check that we are not creating contracts where we cannot
pub(crate) fn check_circular_reference(contract_no: usize, ns: &mut Namespace) {
    let mut creates = Vec::new();
    let mut diagnostics = Diagnostics::default();

    swap(&mut creates, &mut ns.contracts[contract_no].creates);

    for function_no in ns.contracts[contract_no].all_functions.keys() {
        for (loc, no) in &ns.functions[*function_no].creates {
            if contract_no == *no {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "cannot construct current contract '{}'",
                        ns.contracts[*no].id
                    ),
                ));
                continue;
            }

            // check for circular references
            if circular_reference(*no, contract_no, ns) {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "circular reference creating contract '{}'",
                        ns.contracts[*no].id
                    ),
                ));
                continue;
            }

            if !creates.contains(no) {
                creates.push(*no);
            }
        }
    }

    swap(&mut creates, &mut ns.contracts[contract_no].creates);
    ns.diagnostics.extend(diagnostics);
}

/// When calling a constructor on Solana, we must verify it the contract we are instantiating has
/// a program id annotation and require the accounts call argument if the call is inside a loop.
pub(super) fn solana_constructor_check(
    loc: &pt::Loc,
    constructor_contract_no: usize,
    diagnostics: &mut Diagnostics,
    context: &mut ExprContext,
    call_args: &CallArgs,
    ns: &mut Namespace,
) {
    if !ns.contracts[constructor_contract_no].instantiable {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "cannot construct '{}' of type '{}'",
                ns.contracts[constructor_contract_no].id, ns.contracts[constructor_contract_no].ty
            ),
        ));
    }

    if let Some(context_contract) = context.contract_no {
        if circular_reference(constructor_contract_no, context_contract, ns) {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "circular reference creating contract '{}'",
                    ns.contracts[constructor_contract_no].id
                ),
            ));
        }

        if !ns.contracts[context_contract]
            .creates
            .contains(&constructor_contract_no)
        {
            ns.contracts[context_contract]
                .creates
                .push(constructor_contract_no);
        }
    } else {
        diagnostics.push(Diagnostic::error(
            *loc,
            "constructors not allowed in free standing functions".to_string(),
        ));
    }

    if !context.loops.in_a_loop() || !call_args.accounts.is_absent() {
        return;
    }

    if let Some(function_no) = context.function_no {
        if matches!(
            ns.functions[function_no].visibility,
            Visibility::External(_)
        ) {
            diagnostics.push(Diagnostic::error(
                *loc,
                "the {accounts: ..} call argument is needed since the constructor may be \
                called multiple times"
                    .to_string(),
            ));
        }
    }
}
