use super::ast::*;
use super::contracts::is_base;
use super::expression::{
    cast, constructor_named_args, expression, function_call_expr, match_constructor_to_args,
    named_function_call_expr, new, try_cast,
};
use super::symtable::{LoopScopes, Symtable};
use num_bigint::BigInt;
use parser::pt;
use std::collections::HashMap;

pub fn resolve_function_body(
    def: &pt::FunctionDefinition,
    file_no: usize,
    contract_no: usize,
    function_no: usize,
    ns: &mut Namespace,
) -> Result<(), ()> {
    let mut symtable = Symtable::new();
    let mut loops = LoopScopes::new();
    let mut res = Vec::new();

    // first add function parameters
    for (i, p) in def.params.iter().enumerate() {
        let p = p.1.as_ref().unwrap();
        if let Some(ref name) = p.name {
            if let Some(pos) = symtable.add(
                name,
                ns.contracts[contract_no].functions[function_no].params[i]
                    .ty
                    .clone(),
                ns,
            ) {
                ns.check_shadowing(file_no, Some(contract_no), name);

                symtable.arguments.push(Some(pos));
            }
        } else {
            symtable.arguments.push(None);
        }
    }

    // now that the function arguments have been resolved, we can resolve the bases for
    // constructors.
    if def.ty == pt::FunctionTy::Constructor {
        let mut resolve_bases: HashMap<usize, pt::Loc> = HashMap::new();
        let mut all_ok = true;

        for attr in &def.attributes {
            if let pt::FunctionAttribute::BaseArguments(loc, base) = attr {
                match ns.resolve_contract(file_no, &base.name) {
                    Some(base_no) => {
                        if base_no == contract_no || !is_base(base_no, contract_no, ns) {
                            ns.diagnostics.push(Diagnostic::error(
                                *loc,
                                format!(
                                    "contract ‘{}’ is not a base contract of ‘{}’",
                                    base.name.name, ns.contracts[contract_no].name,
                                ),
                            ));
                            all_ok = false;
                        } else if let Some(prev) = resolve_bases.get(&base_no) {
                            ns.diagnostics.push(Diagnostic::error_with_note(
                                *loc,
                                format!("duplicate base contract ‘{}’", base.name.name),
                                *prev,
                                format!("previous base contract ‘{}’", base.name.name),
                            ));
                            all_ok = false;
                        } else if let Some(args) = &base.args {
                            let mut resolved_args = Vec::new();
                            let mut ok = true;

                            for arg in args {
                                if let Ok(e) = expression(
                                    &arg,
                                    file_no,
                                    Some(contract_no),
                                    ns,
                                    &symtable,
                                    false,
                                ) {
                                    resolved_args.push(e);
                                } else {
                                    ok = false;
                                    all_ok = false;
                                }
                            }

                            // find constructor which matches this
                            if ok {
                                if let Ok((Some(constructor_no), args)) =
                                    match_constructor_to_args(&base.loc, resolved_args, base_no, ns)
                                {
                                    ns.contracts[contract_no].functions[function_no]
                                        .bases
                                        .insert(base_no, (base.loc, constructor_no, args));

                                    resolve_bases.insert(base_no, base.loc);
                                }
                            }
                        } else {
                            ns.diagnostics.push(Diagnostic::error(
                                *loc,
                                format!(
                                    "missing arguments to constructor of contract ‘{}’",
                                    base.name.name
                                ),
                            ));
                            all_ok = false;
                        }
                    }
                    None => {
                        if base.args.is_none() {
                            ns.diagnostics.push(Diagnostic::error(
                                *loc,
                                format!("unknown function attribute ‘{}’", base.name.name),
                            ));
                        } else {
                            ns.diagnostics.push(Diagnostic::error(
                                base.name.loc,
                                format!("contract ‘{}’ not found", base.name.name),
                            ));
                        }
                        all_ok = false;
                    }
                }
            }
        }

        if all_ok && ns.contracts[contract_no].is_concrete() {
            for base in &ns.contracts[contract_no].bases {
                // do we have constructor arguments
                if base.constructor.is_some() || resolve_bases.contains_key(&base.contract_no) {
                    continue;
                }

                // does the contract require arguments
                if ns.contracts[base.contract_no].constructor_needs_arguments() {
                    ns.diagnostics.push(Diagnostic::error(
                        def.loc,
                        format!(
                            "missing arguments to contract ‘{}’ constructor",
                            ns.contracts[base.contract_no].name
                        ),
                    ));
                }
            }
        }
    }

    // a function with no return values does not need a return statement
    let mut return_required = !def.returns.is_empty();

    // If any of the return values are named, then the return statement can be omitted at
    // the end of the function, and return values may be omitted too. Create variables to
    // store the return values
    for (i, p) in def.returns.iter().enumerate() {
        let ret = &ns.contracts[contract_no].functions[function_no].returns[i];

        if let Some(ref name) = p.1.as_ref().unwrap().name {
            return_required = false;

            if let Some(pos) = symtable.add(name, ret.ty.clone(), ns) {
                ns.check_shadowing(file_no, Some(contract_no), name);

                symtable.returns.push(pos);
            }
        } else {
            // anonymous return
            let id = pt::Identifier {
                loc: p.0,
                name: "".to_owned(),
            };

            let pos = symtable.add(&id, ret.ty.clone(), ns).unwrap();

            symtable.returns.push(pos);
        }
    }

    if def.body.is_none() {
        return Ok(());
    }

    let body = def.body.as_ref().unwrap();

    let reachable = statement(
        body,
        &mut res,
        file_no,
        contract_no,
        function_no,
        &mut symtable,
        &mut loops,
        ns,
    )?;

    // ensure we have a return instruction
    if reachable {
        if let Some(Statement::Return(_, _)) = res.last() {
            // ok
        } else if return_required {
            ns.diagnostics.push(Diagnostic::error(
                body.loc(),
                "missing return statement".to_string(),
            ));
            return Err(());
        } else {
            // add implicit return
            statement(
                &pt::Statement::Return(pt::Loc(0, 0, 0), None),
                &mut res,
                file_no,
                contract_no,
                function_no,
                &mut symtable,
                &mut loops,
                ns,
            )?;
        }
    }

    ns.contracts[contract_no].functions[function_no].body = res;
    std::mem::swap(
        &mut ns.contracts[contract_no].functions[function_no].symtable,
        &mut symtable,
    );

    Ok(())
}

/// Resolve a statement
fn statement(
    stmt: &pt::Statement,
    res: &mut Vec<Statement>,
    file_no: usize,
    contract_no: usize,
    function_no: usize,
    symtable: &mut Symtable,
    loops: &mut LoopScopes,
    ns: &mut Namespace,
) -> Result<bool, ()> {
    match stmt {
        pt::Statement::VariableDefinition(loc, decl, initializer) => {
            let var_ty = resolve_var_decl_ty(&decl.ty, &decl.storage, file_no, contract_no, ns)?;

            let initializer = if let Some(init) = initializer {
                let expr = expression(init, file_no, Some(contract_no), ns, symtable, false)?;

                Some(cast(&decl.name.loc, expr, &var_ty, true, ns)?)
            } else {
                None
            };

            if let Some(pos) = symtable.add(&decl.name, var_ty.clone(), ns) {
                ns.check_shadowing(file_no, Some(contract_no), &decl.name);

                res.push(Statement::VariableDecl(
                    *loc,
                    pos,
                    Parameter {
                        loc: decl.loc,
                        ty: var_ty,
                        name: decl.name.name.to_owned(),
                        indexed: false,
                    },
                    initializer,
                ));
            }

            Ok(true)
        }
        pt::Statement::Block(_, stmts) => {
            symtable.new_scope();
            let mut reachable = true;

            for stmt in stmts {
                if !reachable {
                    ns.diagnostics.push(Diagnostic::error(
                        stmt.loc(),
                        "unreachable statement".to_string(),
                    ));
                    return Err(());
                }
                reachable = statement(
                    &stmt,
                    res,
                    file_no,
                    contract_no,
                    function_no,
                    symtable,
                    loops,
                    ns,
                )?;
            }

            symtable.leave_scope();

            Ok(reachable)
        }
        pt::Statement::Break(loc) => {
            if loops.do_break() {
                res.push(Statement::Break(*loc));
                Ok(false)
            } else {
                ns.diagnostics.push(Diagnostic::error(
                    stmt.loc(),
                    "break statement not in loop".to_string(),
                ));
                Err(())
            }
        }
        pt::Statement::Continue(loc) => {
            if loops.do_continue() {
                res.push(Statement::Continue(*loc));
                Ok(false)
            } else {
                ns.diagnostics.push(Diagnostic::error(
                    stmt.loc(),
                    "continue statement not in loop".to_string(),
                ));
                Err(())
            }
        }
        pt::Statement::While(loc, cond_expr, body) => {
            let expr = expression(cond_expr, file_no, Some(contract_no), ns, symtable, false)?;

            let cond = cast(&expr.loc(), expr, &Type::Bool, true, ns)?;

            symtable.new_scope();
            let mut body_stmts = Vec::new();
            loops.new_scope();
            statement(
                body,
                &mut body_stmts,
                file_no,
                contract_no,
                function_no,
                symtable,
                loops,
                ns,
            )?;
            symtable.leave_scope();
            loops.leave_scope();

            res.push(Statement::While(*loc, true, cond, body_stmts));

            Ok(true)
        }
        pt::Statement::DoWhile(loc, body, cond_expr) => {
            let expr = expression(cond_expr, file_no, Some(contract_no), ns, symtable, false)?;

            let cond = cast(&expr.loc(), expr, &Type::Bool, true, ns)?;

            symtable.new_scope();
            let mut body_stmts = Vec::new();
            loops.new_scope();
            statement(
                body,
                &mut body_stmts,
                file_no,
                contract_no,
                function_no,
                symtable,
                loops,
                ns,
            )?;
            symtable.leave_scope();
            loops.leave_scope();

            res.push(Statement::DoWhile(*loc, true, body_stmts, cond));
            Ok(true)
        }
        pt::Statement::If(loc, cond_expr, then, else_) => {
            let expr = expression(cond_expr, file_no, Some(contract_no), ns, symtable, false)?;

            let cond = cast(&expr.loc(), expr, &Type::Bool, true, ns)?;

            symtable.new_scope();
            let mut then_stmts = Vec::new();
            let mut reachable = statement(
                then,
                &mut then_stmts,
                file_no,
                contract_no,
                function_no,
                symtable,
                loops,
                ns,
            )?;
            symtable.leave_scope();

            let mut else_stmts = Vec::new();
            if let Some(stmts) = else_ {
                symtable.new_scope();
                reachable &= statement(
                    stmts,
                    &mut else_stmts,
                    file_no,
                    contract_no,
                    function_no,
                    symtable,
                    loops,
                    ns,
                )?;

                symtable.leave_scope();
            } else {
                reachable = true;
            }

            res.push(Statement::If(*loc, reachable, cond, then_stmts, else_stmts));

            Ok(reachable)
        }
        pt::Statement::Args(loc, _) => {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                "expected code block, not list of named arguments".to_string(),
            ));
            Err(())
        }
        pt::Statement::For(loc, init_stmt, None, next_stmt, body_stmt) => {
            symtable.new_scope();

            let mut init = Vec::new();

            if let Some(init_stmt) = init_stmt {
                statement(
                    init_stmt,
                    &mut init,
                    file_no,
                    contract_no,
                    function_no,
                    symtable,
                    loops,
                    ns,
                )?;
            }

            loops.new_scope();

            let mut body = Vec::new();

            if let Some(body_stmt) = body_stmt {
                statement(
                    body_stmt,
                    &mut body,
                    file_no,
                    contract_no,
                    function_no,
                    symtable,
                    loops,
                    ns,
                )?;
            }

            let control = loops.leave_scope();
            let reachable = control.no_breaks > 0;
            let mut next = Vec::new();

            if let Some(next_stmt) = next_stmt {
                statement(
                    next_stmt,
                    &mut next,
                    file_no,
                    contract_no,
                    function_no,
                    symtable,
                    loops,
                    ns,
                )?;
            }

            symtable.leave_scope();

            res.push(Statement::For {
                loc: *loc,
                reachable,
                init,
                next,
                cond: None,
                body,
            });

            Ok(reachable)
        }
        pt::Statement::For(loc, init_stmt, Some(cond_expr), next_stmt, body_stmt) => {
            symtable.new_scope();

            let mut init = Vec::new();
            let mut body = Vec::new();
            let mut next = Vec::new();

            if let Some(init_stmt) = init_stmt {
                statement(
                    init_stmt,
                    &mut init,
                    file_no,
                    contract_no,
                    function_no,
                    symtable,
                    loops,
                    ns,
                )?;
            }

            let expr = expression(cond_expr, file_no, Some(contract_no), ns, symtable, false)?;

            let cond = cast(&cond_expr.loc(), expr, &Type::Bool, true, ns)?;

            // continue goes to next, and if that does exist, cond
            loops.new_scope();

            let mut body_reachable = match body_stmt {
                Some(body_stmt) => statement(
                    body_stmt,
                    &mut body,
                    file_no,
                    contract_no,
                    function_no,
                    symtable,
                    loops,
                    ns,
                )?,
                None => true,
            };

            let control = loops.leave_scope();

            if control.no_continues > 0 {
                body_reachable = true;
            }

            if body_reachable {
                if let Some(next_stmt) = next_stmt {
                    statement(
                        next_stmt,
                        &mut next,
                        file_no,
                        contract_no,
                        function_no,
                        symtable,
                        loops,
                        ns,
                    )?;
                }
            }

            symtable.leave_scope();

            res.push(Statement::For {
                loc: *loc,
                reachable: true,
                init,
                next,
                cond: Some(cond),
                body,
            });

            Ok(true)
        }
        pt::Statement::Return(loc, None) => {
            let no_returns = ns.contracts[contract_no].functions[function_no]
                .returns
                .len();

            if symtable.returns.len() != no_returns {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "missing return value, {} return values expected",
                        no_returns
                    ),
                ));
                return Err(());
            }

            res.push(Statement::Return(
                *loc,
                symtable
                    .returns
                    .iter()
                    .map(|pos| {
                        Expression::Variable(pt::Loc(0, 0, 0), symtable.vars[pos].ty.clone(), *pos)
                    })
                    .collect(),
            ));

            Ok(false)
        }
        pt::Statement::Return(loc, Some(returns)) => {
            let vals = return_with_values(
                returns,
                loc,
                file_no,
                contract_no,
                function_no,
                symtable,
                ns,
            )?;

            res.push(Statement::Return(*loc, vals));

            Ok(false)
        }
        pt::Statement::Expression(loc, expr) => {
            // delete statement
            if let pt::Expression::Delete(_, expr) = expr {
                let expr = expression(expr, file_no, Some(contract_no), ns, symtable, false)?;

                return if let Type::StorageRef(ty) = expr.ty() {
                    if expr.ty().is_mapping() {
                        ns.diagnostics.push(Diagnostic::error(
                            *loc,
                            "‘delete’ cannot be applied to mapping type".to_string(),
                        ));
                        return Err(());
                    }

                    res.push(Statement::Delete(*loc, ty.as_ref().clone(), expr));

                    Ok(true)
                } else {
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        "argument to ‘delete’ should be storage reference".to_string(),
                    ));

                    Err(())
                };
            }

            // is it a destructure statement
            if let pt::Expression::Assign(_, var, expr) = expr {
                if let pt::Expression::List(_, var) = var.as_ref() {
                    res.push(destructure(
                        loc,
                        var,
                        expr,
                        file_no,
                        contract_no,
                        symtable,
                        ns,
                    )?);

                    // if a noreturn function was called, then the destructure would not resolve
                    return Ok(true);
                }
            }

            // the rest
            let expr = expression(expr, file_no, Some(contract_no), ns, symtable, false)?;

            let reachable = expr.ty() != Type::Unreachable;

            res.push(Statement::Expression(*loc, reachable, expr));

            Ok(reachable)
        }
        pt::Statement::Try(loc, expr, returns_and_ok, error_stmt, catch_stmt) => {
            let (stmt, reachable) = try_catch(
                loc,
                expr,
                returns_and_ok,
                error_stmt,
                catch_stmt,
                file_no,
                contract_no,
                function_no,
                symtable,
                loops,
                ns,
            )?;
            res.push(stmt);

            Ok(reachable)
        }
        pt::Statement::Emit(loc, ty) => {
            if let Ok(emit) = emit_event(loc, ty, file_no, contract_no, symtable, ns) {
                res.push(emit);
            }

            Ok(true)
        }
        _ => unreachable!(),
    }
}

/// Resolve emit event
fn emit_event(
    loc: &pt::Loc,
    ty: &pt::Expression,
    file_no: usize,
    contract_no: usize,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<Statement, ()> {
    match ty {
        pt::Expression::FunctionCall(_, ty, args) => {
            let event_no = ns.resolve_event(file_no, Some(contract_no), ty)?;

            let mut resolved_args = Vec::new();

            for arg in args {
                let expr = expression(arg, file_no, Some(contract_no), ns, symtable, false)?;
                resolved_args.push(expr);
            }

            let event = &ns.events[event_no];
            if resolved_args.len() != event.fields.len() {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "event type ‘{}’ has {} fields, {} provided",
                        event.name,
                        event.fields.len(),
                        resolved_args.len()
                    ),
                ));
                return Err(());
            }
            let mut diagnostics = Vec::new();
            let mut cast_args = Vec::new();
            // check if arguments can be implicitly casted
            for (i, arg) in resolved_args.iter().enumerate() {
                match try_cast(&arg.loc(), arg.clone(), &event.fields[i].ty, true, ns) {
                    Ok(expr) => cast_args.push(expr),
                    Err(e) => {
                        diagnostics.push(e);
                    }
                }
            }

            if diagnostics.is_empty() {
                if !ns.contracts[contract_no].sends_events.contains(&event_no) {
                    ns.contracts[contract_no].sends_events.push(event_no);
                }
                return Ok(Statement::Emit {
                    loc: *loc,
                    event_no,
                    args: cast_args,
                });
            } else {
                ns.diagnostics.extend(diagnostics);
            }
        }
        pt::Expression::NamedFunctionCall(_, ty, args) => {
            let event_no = ns.resolve_event(file_no, Some(contract_no), ty)?;

            let mut arguments = HashMap::new();

            for arg in args {
                if arguments.contains_key(&arg.name.name) {
                    ns.diagnostics.push(Diagnostic::error(
                        arg.name.loc,
                        format!("duplicate argument with name ‘{}’", arg.name.name),
                    ));
                    return Err(());
                }
                arguments.insert(
                    arg.name.name.to_string(),
                    expression(&arg.expr, file_no, Some(contract_no), ns, symtable, false)?,
                );
            }

            let event = &ns.events[event_no];
            let params_len = event.fields.len();

            if params_len != arguments.len() {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "event expects {} arguments, {} provided",
                        params_len,
                        args.len()
                    ),
                ));
                return Err(());
            }

            let mut matches = true;
            let mut cast_args = Vec::new();
            let mut diagnostics = Vec::new();

            // check if arguments can be implicitly casted
            for i in 0..params_len {
                let param = event.fields[i].clone();

                if param.name.is_empty() {
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "event ‘{}’ cannot emitted by argument name since argument {} has no name",
                            event.name, i,
                        ),
                    ));
                    matches = false;
                    break;
                }

                let arg = match arguments.get(&param.name) {
                    Some(a) => a,
                    None => {
                        matches = false;
                        ns.diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "missing argument ‘{}’ to event ‘{}’",
                                param.name, event.name,
                            ),
                        ));
                        break;
                    }
                };
                match try_cast(&arg.loc(), arg.clone(), &param.ty, true, ns) {
                    Ok(expr) => cast_args.push(expr),
                    Err(e) => {
                        diagnostics.push(e);
                        matches = false;
                    }
                }
            }

            if matches {
                return Ok(Statement::Emit {
                    loc: *loc,
                    event_no,
                    args: cast_args,
                });
            } else {
                ns.diagnostics.extend(diagnostics);
            }
        }
        pt::Expression::FunctionCallBlock(_, ty, block) => {
            let _ = ns.resolve_event(file_no, Some(contract_no), ty);

            ns.diagnostics.push(Diagnostic::error(
                block.loc(),
                "expected event arguments, found code block".to_string(),
            ));
        }
        _ => unreachable!(),
    }

    Err(())
}

/// Resolve destructuring assignment
fn destructure(
    loc: &pt::Loc,
    vars: &[(pt::Loc, Option<pt::Parameter>)],
    expr: &pt::Expression,
    file_no: usize,
    contract_no: usize,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<Statement, ()> {
    let expr = match expr {
        pt::Expression::FunctionCall(loc, ty, args) => {
            function_call_expr(loc, ty, args, file_no, Some(contract_no), ns, symtable)?
        }
        pt::Expression::NamedFunctionCall(loc, ty, args) => {
            named_function_call_expr(loc, ty, args, file_no, Some(contract_no), ns, symtable)?
        }
        _ => {
            let mut list = Vec::new();

            for e in parameter_list_to_expr_list(expr, ns)? {
                let e = expression(e, file_no, Some(contract_no), ns, symtable, false)?;

                match e.ty() {
                    Type::Void | Type::Unreachable => {
                        ns.diagnostics.push(Diagnostic::error(
                            e.loc(),
                            "function does not return a value".to_string(),
                        ));
                        return Err(());
                    }
                    _ => (),
                }

                list.push(e);
            }

            Expression::List(*loc, list)
        }
    };

    let mut tys = expr.tys();

    // Return type void or unreachable are synthetic
    if tys.len() == 1 && (tys[0] == Type::Unreachable || tys[0] == Type::Void) {
        tys.truncate(0);
    }

    if vars.len() != tys.len() {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "destructuring assignment has {} elements on the left and {} on the right",
                vars.len(),
                tys.len()
            ),
        ));
        return Err(());
    }

    // first resolve the fields
    let mut fields = Vec::new();

    for (i, e) in vars.iter().enumerate() {
        match &e.1 {
            None => {
                fields.push(DestructureField::None);
            }
            Some(pt::Parameter {
                loc,
                ty,
                storage,
                name: None,
            }) => {
                if let Some(storage) = storage {
                    ns.diagnostics.push(Diagnostic::error(
                        *storage.loc(),
                        format!("storage modifier ‘{}’ not permitted on assignment", storage),
                    ));
                    return Err(());
                }

                // ty will just be a normal expression, not a type
                let e = expression(ty, file_no, Some(contract_no), ns, symtable, false)?;

                match &e {
                    Expression::ConstantVariable(_, _, contract_no, var_no) => {
                        ns.diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "cannot assign to constant ‘{}’",
                                ns.contracts[*contract_no].variables[*var_no].name
                            ),
                        ));
                        return Err(());
                    }
                    Expression::StorageVariable(_, _, _, _) | Expression::Variable(_, _, _) => (),
                    _ => match e.ty() {
                        Type::Ref(_) | Type::StorageRef(_) => (),
                        _ => {
                            ns.diagnostics.push(Diagnostic::error(
                                *loc,
                                "expression is not assignable".to_string(),
                            ));
                            return Err(());
                        }
                    },
                }

                // here we only CHECK if we can cast the type
                let _ = cast(
                    &loc,
                    Expression::FunctionArg(*loc, tys[i].clone(), i),
                    e.ty().deref_any(),
                    true,
                    ns,
                )?;

                fields.push(DestructureField::Expression(e));
            }
            Some(pt::Parameter {
                loc,
                ty,
                storage,
                name: Some(name),
            }) => {
                let ty = resolve_var_decl_ty(&ty, &storage, file_no, contract_no, ns)?;

                // here we only CHECK if we can cast the type
                let _ = cast(
                    loc,
                    Expression::FunctionArg(e.0, tys[i].clone(), i),
                    ty.deref_any(),
                    true,
                    ns,
                )?;

                if let Some(pos) = symtable.add(&name, ty.clone(), ns) {
                    ns.check_shadowing(file_no, Some(contract_no), &name);

                    fields.push(DestructureField::VariableDecl(
                        pos,
                        Parameter {
                            loc: *loc,
                            name: name.name.to_owned(),
                            ty,
                            indexed: false,
                        },
                    ));
                }
            }
        }
    }

    Ok(Statement::Destructure(*loc, fields, expr))
}

/// Resolve the type of a variable declaration
fn resolve_var_decl_ty(
    ty: &pt::Expression,
    storage: &Option<pt::StorageLocation>,
    file_no: usize,
    contract_no: usize,
    ns: &mut Namespace,
) -> Result<Type, ()> {
    let mut var_ty = ns.resolve_type(file_no, Some(contract_no), false, &ty)?;

    if let Some(storage) = storage {
        if !var_ty.can_have_data_location() {
            ns.diagnostics.push(Diagnostic::error(
                *storage.loc(),
                format!(
                    "data location ‘{}’ only allowed for array, struct or mapping type",
                    storage
                ),
            ));
            return Err(());
        }

        if let pt::StorageLocation::Storage(_) = storage {
            var_ty = Type::StorageRef(Box::new(var_ty));
        }

        // Note we are completely ignoring memory or calldata data locations. Everything
        // will be stored in memory.
    }

    if var_ty.contains_mapping(ns) && !var_ty.is_contract_storage() {
        ns.diagnostics.push(Diagnostic::error(
            ty.loc(),
            "mapping only allowed in storage".to_string(),
        ));
        return Err(());
    }

    if !var_ty.is_contract_storage() && var_ty.size_hint(ns) > BigInt::from(1024 * 1024) {
        ns.diagnostics.push(Diagnostic::error(
            ty.loc(),
            "type to large to fit into memory".to_string(),
        ));
        return Err(());
    }

    Ok(var_ty)
}

/// Parse return statement with values
fn return_with_values(
    returns: &pt::Expression,
    loc: &pt::Loc,
    file_no: usize,
    contract_no: usize,
    function_no: usize,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<Vec<Expression>, ()> {
    let returns = parameter_list_to_expr_list(returns, ns)?;

    let no_returns = ns.contracts[contract_no].functions[function_no]
        .returns
        .len();

    if no_returns > 0 && returns.is_empty() {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "missing return value, {} return values expected",
                no_returns
            ),
        ));
        return Err(());
    }

    if no_returns == 0 && !returns.is_empty() {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            "function has no return values".to_string(),
        ));
        return Err(());
    }

    if no_returns != returns.len() {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "incorrect number of return values, expected {} but got {}",
                no_returns,
                returns.len()
            ),
        ));
        return Err(());
    }

    let mut exprs = Vec::new();

    for (i, r) in returns.iter().enumerate() {
        let e = expression(r, file_no, Some(contract_no), ns, symtable, false)?;

        exprs.push(cast(
            &r.loc(),
            e,
            &ns.contracts[contract_no].functions[function_no].returns[i]
                .ty
                .clone(),
            true,
            ns,
        )?);
    }

    Ok(exprs)
}

/// The parser generates parameter lists for lists. Sometimes this needs to be a
/// simple expression list.
pub fn parameter_list_to_expr_list<'a>(
    e: &'a pt::Expression,
    ns: &mut Namespace,
) -> Result<Vec<&'a pt::Expression>, ()> {
    if let pt::Expression::List(_, v) = &e {
        let mut list = Vec::new();
        let mut broken = false;

        for e in v {
            match &e.1 {
                None => {
                    ns.diagnostics
                        .push(Diagnostic::error(e.0, "stray comma".to_string()));
                    broken = true;
                }
                Some(pt::Parameter {
                    name: Some(name), ..
                }) => {
                    ns.diagnostics.push(Diagnostic::error(
                        name.loc,
                        "single value expected".to_string(),
                    ));
                    broken = true;
                }
                Some(pt::Parameter {
                    storage: Some(storage),
                    ..
                }) => {
                    ns.diagnostics.push(Diagnostic::error(
                        *storage.loc(),
                        "storage specified not permitted here".to_string(),
                    ));
                    broken = true;
                }
                Some(pt::Parameter { ty, .. }) => {
                    list.push(ty);
                }
            }
        }

        if !broken {
            Ok(list)
        } else {
            Err(())
        }
    } else {
        Ok(vec![e])
    }
}

/// Parse try catch
#[allow(clippy::type_complexity)]
fn try_catch(
    loc: &pt::Loc,
    expr: &pt::Expression,
    returns_and_ok: &Option<(Vec<(pt::Loc, Option<pt::Parameter>)>, Box<pt::Statement>)>,
    error_stmt: &Option<Box<(pt::Identifier, pt::Parameter, pt::Statement)>>,
    catch_stmt: &(pt::Parameter, pt::Statement),
    file_no: usize,
    contract_no: usize,
    function_no: usize,
    symtable: &mut Symtable,
    loops: &mut LoopScopes,
    ns: &mut Namespace,
) -> Result<(Statement, bool), ()> {
    let mut expr = expr;
    let mut ok = None;

    while let pt::Expression::FunctionCallBlock(_, e, block) = expr {
        if ok.is_some() {
            ns.diagnostics.push(Diagnostic::error(
                block.loc(),
                "unexpected code block".to_string(),
            ));
            return Err(());
        }

        ok = Some(block.as_ref());

        expr = e.as_ref();
    }

    let fcall = match expr {
        pt::Expression::FunctionCall(loc, ty, args) => {
            function_call_expr(loc, ty, args, file_no, Some(contract_no), ns, symtable)?
        }
        pt::Expression::NamedFunctionCall(loc, ty, args) => {
            named_function_call_expr(loc, ty, args, file_no, Some(contract_no), ns, symtable)?
        }
        pt::Expression::New(loc, call) => {
            let mut call = call.as_ref();

            while let pt::Expression::FunctionCallBlock(_, expr, block) = call {
                if ok.is_some() {
                    ns.diagnostics.push(Diagnostic::error(
                        block.loc(),
                        "unexpected code block".to_string(),
                    ));
                    return Err(());
                }

                ok = Some(block.as_ref());

                call = expr.as_ref();
            }

            match call {
                pt::Expression::FunctionCall(_, ty, args) => {
                    new(loc, ty, args, file_no, Some(contract_no), ns, symtable)?
                }
                pt::Expression::NamedFunctionCall(_, ty, args) => {
                    constructor_named_args(loc, ty, args, file_no, Some(contract_no), ns, symtable)?
                }
                _ => unreachable!(),
            }
        }
        _ => {
            ns.diagnostics.push(Diagnostic::error(
                expr.loc(),
                "try only supports external calls or constructor calls".to_string(),
            ));
            return Err(());
        }
    };

    let mut returns = &Vec::new();

    if let Some((rets, block)) = returns_and_ok {
        if ok.is_some() {
            ns.diagnostics.push(Diagnostic::error(
                block.loc(),
                "unexpected code block".to_string(),
            ));
            return Err(());
        }

        ok = Some(block);

        returns = rets;
    }

    let ok = match ok {
        Some(ok) => ok,
        None => {
            // position after the expression
            let pos = expr.loc().1;

            ns.diagnostics.push(Diagnostic::error(
                pt::Loc(file_no, pos, pos),
                "code block missing for no catch".to_string(),
            ));
            return Err(());
        }
    };

    symtable.new_scope();

    let mut args = match &fcall {
        Expression::ExternalFunctionCall {
            contract_no,
            function_no,
            ..
        } => {
            let ftype = &ns.contracts[*contract_no].functions[*function_no];

            if returns.len() != ftype.returns.len() {
                ns.diagnostics.push(Diagnostic::error(
                    expr.loc(),
                    format!(
                        "try returns list has {} entries while function returns {} values",
                        ftype.returns.len(),
                        returns.len()
                    ),
                ));
                return Err(());
            }

            ftype.returns.iter().map(|ret| ret.ty.clone()).collect()
        }
        Expression::Constructor { contract_no, .. } => match returns.len() {
            0 => Vec::new(),
            1 => vec![Type::Contract(*contract_no)],
            _ => {
                ns.diagnostics.push(Diagnostic::error(
                    expr.loc(),
                    format!(
                        "constructor returns single contract, not {} values",
                        returns.len()
                    ),
                ));
                return Err(());
            }
        },
        _ => {
            ns.diagnostics.push(Diagnostic::error(
                expr.loc(),
                "try only supports external calls or constructor calls".to_string(),
            ));
            return Err(());
        }
    };

    symtable.new_scope();

    let mut params = Vec::new();
    let mut broken = false;
    for param in returns {
        let arg_ty = args.remove(0);

        match &param.1 {
            Some(pt::Parameter {
                ty, storage, name, ..
            }) => {
                let ret_ty = resolve_var_decl_ty(&ty, &storage, file_no, contract_no, ns)?;

                if arg_ty != ret_ty {
                    ns.diagnostics.push(Diagnostic::error(
                        ty.loc(),
                        format!(
                            "type ‘{}’ does not match return value of function ‘{}’",
                            ret_ty.to_string(ns),
                            arg_ty.to_string(ns)
                        ),
                    ));
                    broken = true;
                }

                if let Some(name) = name {
                    if let Some(pos) = symtable.add(&name, ret_ty.clone(), ns) {
                        ns.check_shadowing(file_no, Some(contract_no), &name);
                        params.push((
                            Some(pos),
                            Parameter {
                                loc: param.0,
                                ty: ret_ty,
                                name: name.name.to_string(),
                                indexed: false,
                            },
                        ));
                    }
                } else {
                    params.push((
                        None,
                        Parameter {
                            loc: param.0,
                            ty: ret_ty,
                            indexed: false,
                            name: "".to_string(),
                        },
                    ));
                }
            }
            None => {
                ns.diagnostics.push(Diagnostic::error(
                    param.0,
                    "missing return type".to_string(),
                ));
                broken = true;
            }
        }
    }

    if broken {
        return Err(());
    }

    let mut ok_resolved = Vec::new();

    let mut finally_reachable = statement(
        &ok,
        &mut ok_resolved,
        file_no,
        contract_no,
        function_no,
        symtable,
        loops,
        ns,
    )?;

    symtable.leave_scope();

    let error_resolved = if let Some(error_stmt) = error_stmt {
        if error_stmt.0.name != "Error" {
            ns.diagnostics.push(Diagnostic::error(
                error_stmt.0.loc,
                format!(
                    "only catch ‘Error’ is supported, not ‘{}’",
                    error_stmt.0.name
                ),
            ));
            return Err(());
        }

        let error_ty = resolve_var_decl_ty(
            &error_stmt.1.ty,
            &error_stmt.1.storage,
            file_no,
            contract_no,
            ns,
        )?;

        if error_ty != Type::String {
            ns.diagnostics.push(Diagnostic::error(
                error_stmt.1.ty.loc(),
                format!(
                    "catch Error(...) can only take ‘string memory’, not ‘{}’",
                    error_ty.to_string(ns)
                ),
            ));
        }

        symtable.new_scope();

        let mut error_pos = None;
        let mut error_stmt_resolved = Vec::new();
        let mut error_param = Parameter {
            loc: error_stmt.0.loc,
            ty: Type::String,
            name: "".to_string(),
            indexed: false,
        };

        if let Some(name) = &error_stmt.1.name {
            if let Some(pos) = symtable.add(&name, Type::String, ns) {
                ns.check_shadowing(file_no, Some(contract_no), &name);

                error_pos = Some(pos);
                error_param.name = name.name.to_string();
            }
        }

        let reachable = statement(
            &error_stmt.2,
            &mut error_stmt_resolved,
            file_no,
            contract_no,
            function_no,
            symtable,
            loops,
            ns,
        )?;

        finally_reachable &= reachable;

        symtable.leave_scope();

        Some((error_pos, error_param, error_stmt_resolved))
    } else {
        None
    };

    let catch_ty = resolve_var_decl_ty(
        &catch_stmt.0.ty,
        &catch_stmt.0.storage,
        file_no,
        contract_no,
        ns,
    )?;

    if catch_ty != Type::DynamicBytes {
        ns.diagnostics.push(Diagnostic::error(
            catch_stmt.0.ty.loc(),
            format!(
                "catch can only take ‘bytes memory’, not ‘{}’",
                catch_ty.to_string(ns)
            ),
        ));
        return Err(());
    }

    symtable.new_scope();

    let mut catch_param = Parameter {
        loc: catch_stmt.0.loc,
        ty: Type::DynamicBytes,
        name: "".to_owned(),
        indexed: false,
    };
    let mut catch_param_pos = None;
    let mut catch_stmt_resolved = Vec::new();

    if let Some(name) = &catch_stmt.0.name {
        if let Some(pos) = symtable.add(&name, catch_ty, ns) {
            ns.check_shadowing(file_no, Some(contract_no), &name);
            catch_param_pos = Some(pos);
            catch_param.name = name.name.to_string();
        }
    }

    let reachable = statement(
        &catch_stmt.1,
        &mut catch_stmt_resolved,
        file_no,
        contract_no,
        function_no,
        symtable,
        loops,
        ns,
    )?;

    finally_reachable &= reachable;

    symtable.leave_scope();

    let stmt = Statement::TryCatch {
        loc: *loc,
        expr: fcall,
        reachable: finally_reachable,
        returns: params,
        error: error_resolved,
        ok_stmt: ok_resolved,
        catch_param,
        catch_param_pos,
        catch_stmt: catch_stmt_resolved,
    };

    Ok((stmt, finally_reachable))
}
