// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{ArrayLength, CallArgs, Expression, Namespace, RetrieveType, Type};
use crate::sema::diagnostics::Diagnostics;
use crate::sema::expression::{
    collect_call_args, expression, parse_call_args, ExprContext, ResolveTo,
};
use crate::sema::symtable::Symtable;
use crate::sema::unused_variable::used_variable;
use crate::Target;
use solang_parser::diagnostics::Diagnostic;
use solang_parser::pt;
use solang_parser::pt::CodeLocation;
use std::collections::BTreeMap;

/// Resolve an new contract expression with positional arguments
fn constructor(
    loc: &pt::Loc,
    no: usize,
    args: &[pt::Expression],
    call_args: CallArgs,
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    // The current contract cannot be constructed with new. In order to create
    // the contract, we need the code hash of the contract. Part of that code
    // will be code we're emitted here. So we end up with a crypto puzzle.
    let context_contract_no = match context.contract_no {
        Some(n) if n == no => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "new cannot construct current contract '{}'",
                    ns.contracts[no].name
                ),
            ));
            return Err(());
        }
        Some(n) => n,
        None => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "new contract not allowed in this context".to_string(),
            ));
            return Err(());
        }
    };

    if !ns.contracts[no].instantiable {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "cannot construct '{}' of type '{}'",
                ns.contracts[no].name, ns.contracts[no].ty
            ),
        ));

        return Err(());
    }

    if ns.target == Target::Solana && ns.contracts[no].program_id.is_none() {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "in order to instantiate contract '{}', a @program_id is required on contract '{}'",
                ns.contracts[no].name, ns.contracts[no].name
            ),
        ));
    }

    // check for circular references
    if circular_reference(no, context_contract_no, ns) {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "circular reference creating contract '{}'",
                ns.contracts[no].name
            ),
        ));
        return Err(());
    }

    if !ns.contracts[context_contract_no].creates.contains(&no) {
        ns.contracts[context_contract_no].creates.push(no);
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
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<(Option<usize>, Vec<Expression>), ()> {
    let mut errors = Diagnostics::default();

    // constructor call
    let function_nos: Vec<usize> = ns.contracts[contract_no]
        .functions
        .iter()
        .filter(|function_no| ns.functions[**function_no].is_constructor())
        .copied()
        .collect();

    for function_no in &function_nos {
        let mut matches = true;

        let params_len = ns.functions[*function_no].params.len();

        if params_len != args.len() {
            errors.push(Diagnostic::cast_error(
                *loc,
                format!(
                    "constructor expects {} arguments, {} provided",
                    params_len,
                    args.len()
                ),
            ));
            matches = false;
        }

        let mut cast_args = Vec::new();

        // resolve arguments for this constructor
        for (i, arg) in args.iter().enumerate() {
            let ty = ns.functions[*function_no]
                .params
                .get(i)
                .map(|p| p.ty.clone());

            let arg = match expression(
                arg,
                context,
                ns,
                symtable,
                &mut errors,
                if let Some(ty) = &ty {
                    ResolveTo::Type(ty)
                } else {
                    ResolveTo::Unknown
                },
            ) {
                Ok(v) => v,
                Err(()) => {
                    matches = false;
                    continue;
                }
            };

            if let Some(ty) = &ty {
                match arg.cast(&arg.loc(), ty, true, ns, &mut errors) {
                    Ok(expr) => cast_args.push(expr),
                    Err(()) => {
                        matches = false;
                    }
                }
            }
        }

        if matches {
            return Ok((Some(*function_no), cast_args));
        } else if function_nos.len() > 1 && diagnostics.extend_non_casting(&errors) {
            return Err(());
        }
    }

    match function_nos.len() {
        0 if args.is_empty() => {
            return Ok((None, Vec::new()));
        }
        0 | 1 => {
            diagnostics.extend(errors);
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "cannot find overloaded constructor which matches signature".to_string(),
            ));
        }
    }

    Err(())
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
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Diagnostics,
) -> Result<Expression, ()> {
    let (ty, call_args, _) = collect_call_args(ty, diagnostics)?;

    let call_args = parse_call_args(loc, &call_args, false, context, ns, symtable, diagnostics)?;

    let no = match ns.resolve_type(context.file_no, context.contract_no, false, ty, diagnostics)? {
        Type::Contract(n) => n,
        _ => {
            diagnostics.push(Diagnostic::error(*loc, "contract expected".to_string()));
            return Err(());
        }
    };

    // The current contract cannot be constructed with new. In order to create
    // the contract, we need the code hash of the contract. Part of that code
    // will be code we're emitted here. So we end up with a crypto puzzle.
    let context_contract_no = match context.contract_no {
        Some(n) if n == no => {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "new cannot construct current contract '{}'",
                    ns.contracts[no].name
                ),
            ));
            return Err(());
        }
        Some(n) => n,
        None => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "new contract not allowed in this context".to_string(),
            ));
            return Err(());
        }
    };

    if !ns.contracts[no].instantiable {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "cannot construct '{}' of type '{}'",
                ns.contracts[no].name, ns.contracts[no].ty
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
                ns.contracts[no].name
            ),
        ));
        return Err(());
    }

    if !ns.contracts[context_contract_no].creates.contains(&no) {
        ns.contracts[context_contract_no].creates.push(no);
    }

    let mut arguments: BTreeMap<&str, &pt::Expression> = BTreeMap::new();

    for arg in args {
        if let Some(prev) = arguments.get(arg.name.name.as_str()) {
            diagnostics.push(Diagnostic::error_with_note(
                *loc,
                format!("duplicate argument name '{}'", arg.name.name),
                prev.loc(),
                String::from("location of previous argument"),
            ));
            return Err(());
        }
        arguments.insert(&arg.name.name, &arg.expr);
    }

    let mut errors = Diagnostics::default();

    // constructor call
    let function_nos: Vec<usize> = ns.contracts[no]
        .functions
        .iter()
        .filter(|function_no| ns.functions[**function_no].is_constructor())
        .copied()
        .collect();

    // constructor call
    for function_no in &function_nos {
        let func = &ns.functions[*function_no];
        let params_len = func.params.len();

        let mut matches = true;

        let unnamed_params = func.params.iter().filter(|p| p.id.is_none()).count();

        if unnamed_params > 0 {
            errors.push(Diagnostic::cast_error_with_note(
                *loc,
                format!(
                    "constructor cannot be called with named arguments as {} of its parameters do not have names",
                    unnamed_params,
                ),
                func.loc,
                format!("definition of {}", func.ty),
            ));
            matches = false;
        } else if params_len != args.len() {
            errors.push(Diagnostic::cast_error_with_note(
                *loc,
                format!(
                    "constructor expects {} arguments, {} provided",
                    params_len,
                    args.len()
                ),
                func.loc,
                "definition of constructor".to_owned(),
            ));
            matches = false;
        }

        let mut cast_args = Vec::new();

        let func_loc = ns.functions[*function_no].loc;

        // check if arguments can be implicitly casted
        for i in 0..params_len {
            let param = ns.functions[*function_no].params[i].clone();

            let arg = match arguments.get(param.name_as_str()) {
                Some(a) => a,
                None => {
                    matches = false;
                    errors.push(Diagnostic::cast_error_with_note(
                        *loc,
                        format!("missing argument '{}' to constructor", param.name_as_str()),
                        func_loc,
                        "definition of constructor".to_owned(),
                    ));
                    break;
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
                }
            }
        }

        if matches {
            return Ok(Expression::Constructor {
                loc: *loc,
                contract_no: no,
                constructor_no: Some(*function_no),
                args: cast_args,
                call_args,
            });
        } else if function_nos.len() > 1 && diagnostics.extend_non_casting(&errors) {
            return Err(());
        }
    }

    match function_nos.len() {
        0 if args.is_empty() => Ok(Expression::Constructor {
            loc: *loc,
            contract_no: no,
            constructor_no: None,
            args: Vec::new(),
            call_args,
        }),
        0 | 1 => {
            diagnostics.extend(errors);

            Err(())
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                *loc,
                "cannot find overloaded constructor which matches signature".to_string(),
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
    context: &ExprContext,
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

    let ty = ns.resolve_type(context.file_no, context.contract_no, false, ty, diagnostics)?;

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
            let call_args =
                parse_call_args(loc, &call_args, false, context, ns, symtable, diagnostics)?;

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
