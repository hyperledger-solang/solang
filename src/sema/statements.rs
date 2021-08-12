use super::ast::*;
use super::contracts::is_base;
use super::expression::{
    available_functions, call_position_args, cast, constructor_named_args, expression,
    function_call_expr, match_constructor_to_args, named_function_call_expr, new,
};
use super::symtable::{LoopScopes, Symtable};
use crate::parser::pt;
use crate::sema::symtable::VariableUsage;
use crate::sema::unused_variable::{assigned_variable, check_function_call, used_variable};
use std::collections::HashMap;

pub fn resolve_function_body(
    def: &pt::FunctionDefinition,
    file_no: usize,
    contract_no: Option<usize>,
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
                ns.functions[function_no].params[i].ty.clone(),
                ns,
                None,
                VariableUsage::Parameter,
                p.storage.clone(),
            ) {
                ns.check_shadowing(file_no, contract_no, name);

                symtable.arguments.push(Some(pos));
            }
        } else {
            symtable.arguments.push(None);
        }
    }

    // now that the function arguments have been resolved, we can resolve the bases for
    // constructors.
    if def.ty == pt::FunctionTy::Constructor {
        let contract_no = contract_no.unwrap();
        let mut resolve_bases: HashMap<usize, pt::Loc> = HashMap::new();
        let mut all_ok = true;

        for attr in &def.attributes {
            if let pt::FunctionAttribute::BaseOrModifier(loc, base) = attr {
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
                            let mut diagnostics = Vec::new();

                            // find constructor which matches this
                            if let Ok((Some(constructor_no), args)) = match_constructor_to_args(
                                &base.loc,
                                args,
                                file_no,
                                base_no,
                                Some(function_no),
                                contract_no,
                                false,
                                ns,
                                &mut symtable,
                                &mut diagnostics,
                            ) {
                                for arg in &args {
                                    used_variable(ns, arg, &mut symtable);
                                }
                                ns.functions[function_no]
                                    .bases
                                    .insert(base_no, (base.loc, constructor_no, args));

                                resolve_bases.insert(base_no, base.loc);
                            }

                            ns.diagnostics.extend(diagnostics);
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
                if ns.contracts[base.contract_no].constructor_needs_arguments(ns) {
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

    // resolve modifiers on functions
    if def.ty == pt::FunctionTy::Function {
        let mut modifiers = Vec::new();
        let mut diagnostics = Vec::new();

        for attr in &def.attributes {
            if let pt::FunctionAttribute::BaseOrModifier(_, modifier) = attr {
                if let Ok(e) = call_position_args(
                    &modifier.loc,
                    &modifier.name,
                    pt::FunctionTy::Modifier,
                    modifier.args.as_ref().unwrap_or(&Vec::new()),
                    file_no,
                    available_functions(&modifier.name.name, false, file_no, contract_no, ns),
                    true,
                    contract_no,
                    Some(function_no),
                    false,
                    ns,
                    &mut symtable,
                    &mut diagnostics,
                ) {
                    modifiers.push(e);
                }
            }
        }

        ns.diagnostics.extend(diagnostics);
        ns.functions[function_no].modifiers = modifiers;
    }

    // a function with no return values does not need a return statement
    let mut return_required = !def.returns.is_empty();

    // If any of the return values are named, then the return statement can be omitted at
    // the end of the function, and return values may be omitted too. Create variables to
    // store the return values
    for (i, p) in def.returns.iter().enumerate() {
        let ret = &ns.functions[function_no].returns[i];

        if let Some(ref name) = p.1.as_ref().unwrap().name {
            return_required = false;

            if let Some(pos) = symtable.add(
                name,
                ret.ty.clone(),
                ns,
                None,
                VariableUsage::ReturnVariable,
                None,
            ) {
                ns.check_shadowing(file_no, contract_no, name);
                symtable.returns.push(pos);
            }
        } else {
            // anonymous return
            let id = pt::Identifier {
                loc: p.0,
                name: "".to_owned(),
            };

            let pos = symtable
                .add(
                    &id,
                    ret.ty.clone(),
                    ns,
                    None,
                    VariableUsage::AnonymousReturnVariable,
                    None,
                )
                .unwrap();

            symtable.returns.push(pos);
        }
    }

    if def.body.is_none() {
        return Ok(());
    }

    let body = def.body.as_ref().unwrap();

    let mut diagnostics = Vec::new();

    let reachable = statement(
        body,
        &mut res,
        file_no,
        contract_no,
        function_no,
        false,
        &mut symtable,
        &mut loops,
        ns,
        &mut diagnostics,
    );

    ns.diagnostics.extend(diagnostics);

    if reachable? {
        // ensure we have a return instruction
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
                false,
                &mut symtable,
                &mut loops,
                ns,
                &mut Vec::new(),
            )?;
        }
    }

    if def.ty == pt::FunctionTy::Modifier {
        let mut has_underscore = false;

        // unsure modifier has underscore
        fn check_statement(stmt: &Statement, has_underscore: &mut bool) -> bool {
            if stmt.is_underscore() {
                *has_underscore = true;
                false
            } else {
                true
            }
        }

        for stmt in &mut res {
            stmt.recurse(&mut has_underscore, check_statement);
        }

        if !has_underscore {
            ns.diagnostics.push(Diagnostic::error(
                body.loc(),
                "missing ‘_’ in modifier".to_string(),
            ));
        }
    }

    ns.functions[function_no].body = res;

    std::mem::swap(&mut ns.functions[function_no].symtable, &mut symtable);

    Ok(())
}

/// Resolve a statement
fn statement(
    stmt: &pt::Statement,
    res: &mut Vec<Statement>,
    file_no: usize,
    contract_no: Option<usize>,
    function_no: usize,
    unchecked: bool,
    symtable: &mut Symtable,
    loops: &mut LoopScopes,
    ns: &mut Namespace,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<bool, ()> {
    match stmt {
        pt::Statement::VariableDefinition(loc, decl, initializer) => {
            let (var_ty, ty_loc) = resolve_var_decl_ty(
                &decl.ty,
                &decl.storage,
                file_no,
                contract_no,
                ns,
                diagnostics,
            )?;

            let initializer = if let Some(init) = initializer {
                let expr = expression(
                    init,
                    file_no,
                    contract_no,
                    Some(function_no),
                    ns,
                    symtable,
                    false,
                    unchecked,
                    diagnostics,
                    Some(&var_ty),
                )?;

                used_variable(ns, &expr, symtable);

                Some(cast(&expr.loc(), expr, &var_ty, true, ns, diagnostics)?)
            } else {
                None
            };

            if let Some(pos) = symtable.add(
                &decl.name,
                var_ty.clone(),
                ns,
                initializer.clone(),
                VariableUsage::LocalVariable,
                decl.storage.clone(),
            ) {
                ns.check_shadowing(file_no, contract_no, &decl.name);

                res.push(Statement::VariableDecl(
                    *loc,
                    pos,
                    Parameter {
                        loc: decl.loc,
                        ty: var_ty,
                        ty_loc,
                        name: decl.name.name.to_owned(),
                        name_loc: Some(decl.name.loc),
                        indexed: false,
                    },
                    initializer,
                ));
            }

            Ok(true)
        }
        pt::Statement::Block {
            statements,
            unchecked: this_block_unchecked,
            ..
        } => {
            symtable.new_scope();
            let mut reachable = true;

            for stmt in statements {
                if !reachable {
                    ns.diagnostics.push(Diagnostic::error(
                        stmt.loc(),
                        "unreachable statement".to_string(),
                    ));
                    return Err(());
                }
                reachable = statement(
                    stmt,
                    res,
                    file_no,
                    contract_no,
                    function_no,
                    unchecked || *this_block_unchecked,
                    symtable,
                    loops,
                    ns,
                    diagnostics,
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
                diagnostics.push(Diagnostic::error(
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
                diagnostics.push(Diagnostic::error(
                    stmt.loc(),
                    "continue statement not in loop".to_string(),
                ));
                Err(())
            }
        }
        pt::Statement::While(loc, cond_expr, body) => {
            let expr = expression(
                cond_expr,
                file_no,
                contract_no,
                Some(function_no),
                ns,
                symtable,
                false,
                unchecked,
                diagnostics,
                Some(&Type::Bool),
            )?;
            used_variable(ns, &expr, symtable);
            let cond = cast(&expr.loc(), expr, &Type::Bool, true, ns, diagnostics)?;

            symtable.new_scope();
            let mut body_stmts = Vec::new();
            loops.new_scope();
            statement(
                body,
                &mut body_stmts,
                file_no,
                contract_no,
                function_no,
                unchecked,
                symtable,
                loops,
                ns,
                diagnostics,
            )?;
            symtable.leave_scope();
            loops.leave_scope();

            res.push(Statement::While(*loc, true, cond, body_stmts));

            Ok(true)
        }
        pt::Statement::DoWhile(loc, body, cond_expr) => {
            let expr = expression(
                cond_expr,
                file_no,
                contract_no,
                Some(function_no),
                ns,
                symtable,
                false,
                unchecked,
                diagnostics,
                Some(&Type::Bool),
            )?;
            used_variable(ns, &expr, symtable);
            let cond = cast(&expr.loc(), expr, &Type::Bool, true, ns, diagnostics)?;

            symtable.new_scope();
            let mut body_stmts = Vec::new();
            loops.new_scope();
            statement(
                body,
                &mut body_stmts,
                file_no,
                contract_no,
                function_no,
                unchecked,
                symtable,
                loops,
                ns,
                diagnostics,
            )?;
            symtable.leave_scope();
            loops.leave_scope();

            res.push(Statement::DoWhile(*loc, true, body_stmts, cond));
            Ok(true)
        }
        pt::Statement::If(loc, cond_expr, then, else_) => {
            let expr = expression(
                cond_expr,
                file_no,
                contract_no,
                Some(function_no),
                ns,
                symtable,
                false,
                unchecked,
                diagnostics,
                Some(&Type::Bool),
            )?;
            used_variable(ns, &expr, symtable);

            let cond = cast(&expr.loc(), expr, &Type::Bool, true, ns, diagnostics)?;

            symtable.new_scope();
            let mut then_stmts = Vec::new();
            let mut reachable = statement(
                then,
                &mut then_stmts,
                file_no,
                contract_no,
                function_no,
                unchecked,
                symtable,
                loops,
                ns,
                diagnostics,
            )?;
            symtable.leave_scope();

            let mut else_stmts = Vec::new();
            if let Some(stmts) = else_ {
                symtable.new_scope();
                reachable |= statement(
                    stmts,
                    &mut else_stmts,
                    file_no,
                    contract_no,
                    function_no,
                    unchecked,
                    symtable,
                    loops,
                    ns,
                    diagnostics,
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
                    unchecked,
                    symtable,
                    loops,
                    ns,
                    diagnostics,
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
                    unchecked,
                    symtable,
                    loops,
                    ns,
                    diagnostics,
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
                    unchecked,
                    symtable,
                    loops,
                    ns,
                    diagnostics,
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
                    unchecked,
                    symtable,
                    loops,
                    ns,
                    diagnostics,
                )?;
            }

            let expr = expression(
                cond_expr,
                file_no,
                contract_no,
                Some(function_no),
                ns,
                symtable,
                false,
                unchecked,
                diagnostics,
                Some(&Type::Bool),
            )?;

            let cond = cast(&cond_expr.loc(), expr, &Type::Bool, true, ns, diagnostics)?;

            // continue goes to next, and if that does exist, cond
            loops.new_scope();

            let mut body_reachable = match body_stmt {
                Some(body_stmt) => statement(
                    body_stmt,
                    &mut body,
                    file_no,
                    contract_no,
                    function_no,
                    unchecked,
                    symtable,
                    loops,
                    ns,
                    diagnostics,
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
                        unchecked,
                        symtable,
                        loops,
                        ns,
                        diagnostics,
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
            let no_returns = ns.functions[function_no].returns.len();

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
                unchecked,
                symtable,
                ns,
                diagnostics,
            )?;

            for offset in symtable.returns.iter() {
                let elem = symtable.vars.get_mut(offset).unwrap();
                (*elem).assigned = true;
            }

            for item in &vals {
                used_variable(ns, item, symtable);
            }

            res.push(Statement::Return(*loc, vals));

            Ok(false)
        }
        pt::Statement::Expression(loc, expr) => {
            // delete statement
            if let pt::Expression::Delete(_, expr) = expr {
                let expr = expression(
                    expr,
                    file_no,
                    contract_no,
                    Some(function_no),
                    ns,
                    symtable,
                    false,
                    unchecked,
                    diagnostics,
                    None,
                )?;
                used_variable(ns, &expr, symtable);
                return if let Type::StorageRef(_, ty) = expr.ty() {
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

            // is it an underscore modifier statement
            if let pt::Expression::Variable(id) = expr {
                if id.name == "_" {
                    return if ns.functions[function_no].ty == pt::FunctionTy::Modifier {
                        res.push(Statement::Underscore(*loc));

                        Ok(true)
                    } else {
                        ns.diagnostics.push(Diagnostic::error(
                            *loc,
                            "underscore statement only permitted in modifiers".to_string(),
                        ));
                        Err(())
                    };
                }
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
                        function_no,
                        unchecked,
                        symtable,
                        ns,
                        diagnostics,
                    )?);

                    // if a noreturn function was called, then the destructure would not resolve
                    return Ok(true);
                }
            }

            // the rest
            let expr = expression(
                expr,
                file_no,
                contract_no,
                Some(function_no),
                ns,
                symtable,
                false,
                unchecked,
                diagnostics,
                None,
            )?;

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
                unchecked,
                symtable,
                loops,
                ns,
                diagnostics,
            )?;
            res.push(stmt);

            Ok(reachable)
        }
        pt::Statement::Emit(loc, ty) => {
            if let Ok(emit) = emit_event(
                loc,
                ty,
                file_no,
                contract_no,
                function_no,
                unchecked,
                symtable,
                ns,
                diagnostics,
            ) {
                res.push(emit);
            }

            Ok(true)
        }
    }
}

/// Resolve emit event
fn emit_event(
    loc: &pt::Loc,
    ty: &pt::Expression,
    file_no: usize,
    contract_no: Option<usize>,
    function_no: usize,
    unchecked: bool,
    symtable: &mut Symtable,
    ns: &mut Namespace,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Statement, ()> {
    match ty {
        pt::Expression::FunctionCall(_, ty, args) => {
            let event_loc = ty.loc();
            let event_nos = ns.resolve_event(file_no, contract_no, ty, diagnostics)?;

            let mut temp_diagnostics = Vec::new();

            for event_no in &event_nos {
                let event = &mut ns.events[*event_no];
                event.used = true;
                if args.len() != event.fields.len() {
                    temp_diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "event type ‘{}’ has {} fields, {} provided",
                            event.name,
                            event.fields.len(),
                            args.len()
                        ),
                    ));
                    continue;
                }
                let mut cast_args = Vec::new();

                let mut matches = true;
                // check if arguments can be implicitly casted
                for (i, arg) in args.iter().enumerate() {
                    let ty = ns.events[*event_no].fields[i].ty.clone();

                    let arg = match expression(
                        arg,
                        file_no,
                        contract_no,
                        Some(function_no),
                        ns,
                        symtable,
                        false,
                        unchecked,
                        &mut temp_diagnostics,
                        Some(&ty),
                    ) {
                        Ok(e) => e,
                        Err(()) => {
                            matches = false;
                            break;
                        }
                    };
                    used_variable(ns, &arg, symtable);

                    match cast(
                        &arg.loc(),
                        arg.clone(),
                        &ty,
                        true,
                        ns,
                        &mut temp_diagnostics,
                    ) {
                        Ok(expr) => cast_args.push(expr),
                        Err(_) => {
                            matches = false;
                        }
                    }
                }

                if matches {
                    if !ns.functions[function_no].emits_events.contains(event_no) {
                        ns.functions[function_no].emits_events.push(*event_no);
                    }

                    return Ok(Statement::Emit {
                        loc: *loc,
                        event_no: *event_no,
                        event_loc,
                        args: cast_args,
                    });
                }
            }

            if event_nos.len() == 1 {
                diagnostics.extend(temp_diagnostics);
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "cannot find event which matches signature".to_string(),
                ));
            }
        }
        pt::Expression::NamedFunctionCall(_, ty, args) => {
            let event_loc = ty.loc();
            let event_nos = ns.resolve_event(file_no, contract_no, ty, diagnostics)?;

            let mut arguments = HashMap::new();

            for arg in args {
                if arguments.contains_key(&arg.name.name) {
                    ns.diagnostics.push(Diagnostic::error(
                        arg.name.loc,
                        format!("duplicate argument with name ‘{}’", arg.name.name),
                    ));
                    return Err(());
                }
                arguments.insert(&arg.name.name, &arg.expr);
            }

            let mut temp_diagnostics = Vec::new();

            for event_no in &event_nos {
                let event = &mut ns.events[*event_no];
                event.used = true;
                let params_len = event.fields.len();

                if params_len != arguments.len() {
                    temp_diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "event expects {} arguments, {} provided",
                            params_len,
                            args.len()
                        ),
                    ));
                    continue;
                }

                let mut matches = true;
                let mut cast_args = Vec::new();

                // check if arguments can be implicitly casted
                for i in 0..params_len {
                    let param = ns.events[*event_no].fields[i].clone();

                    if param.name.is_empty() {
                        temp_diagnostics.push(Diagnostic::error(
                        *loc,
                        format!(
                            "event ‘{}’ cannot emitted by argument name since argument {} has no name",
                            ns.events[*event_no].name, i,
                        ),
                    ));
                        matches = false;
                        break;
                    }

                    let arg = match arguments.get(&param.name) {
                        Some(a) => a,
                        None => {
                            matches = false;
                            temp_diagnostics.push(Diagnostic::error(
                                *loc,
                                format!(
                                    "missing argument ‘{}’ to event ‘{}’",
                                    param.name, ns.events[*event_no].name,
                                ),
                            ));
                            break;
                        }
                    };

                    let arg = match expression(
                        arg,
                        file_no,
                        contract_no,
                        Some(function_no),
                        ns,
                        symtable,
                        false,
                        unchecked,
                        &mut temp_diagnostics,
                        Some(&param.ty),
                    ) {
                        Ok(e) => e,
                        Err(()) => {
                            matches = false;
                            break;
                        }
                    };

                    used_variable(ns, &arg, symtable);

                    match cast(&arg.loc(), arg, &param.ty, true, ns, &mut temp_diagnostics) {
                        Ok(expr) => cast_args.push(expr),
                        Err(_) => {
                            matches = false;
                        }
                    }
                }

                if matches {
                    if !ns.functions[function_no].emits_events.contains(event_no) {
                        ns.functions[function_no].emits_events.push(*event_no);
                    }

                    return Ok(Statement::Emit {
                        loc: *loc,
                        event_no: *event_no,
                        event_loc,
                        args: cast_args,
                    });
                }
            }

            if event_nos.len() == 1 {
                diagnostics.extend(temp_diagnostics);
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "cannot find event which matches signature".to_string(),
                ));
            }
        }
        pt::Expression::FunctionCallBlock(_, ty, block) => {
            let _ = ns.resolve_event(file_no, contract_no, ty, diagnostics);

            diagnostics.push(Diagnostic::error(
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
    contract_no: Option<usize>,
    function_no: usize,
    unchecked: bool,
    symtable: &mut Symtable,
    ns: &mut Namespace,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Statement, ()> {
    // first resolve the fields so we know the types
    let mut fields = Vec::new();
    let mut left_tys = Vec::new();

    for (_, param) in vars {
        match param {
            None => {
                left_tys.push(None);
                fields.push(DestructureField::None);
            }
            Some(pt::Parameter {
                loc,
                ty,
                storage,
                name: None,
            }) => {
                if let Some(storage) = storage {
                    diagnostics.push(Diagnostic::error(
                        *storage.loc(),
                        format!("storage modifier ‘{}’ not permitted on assignment", storage),
                    ));
                    return Err(());
                }

                // ty will just be a normal expression, not a type
                let e = expression(
                    ty,
                    file_no,
                    contract_no,
                    Some(function_no),
                    ns,
                    symtable,
                    false,
                    unchecked,
                    diagnostics,
                    None,
                )?;

                match &e {
                    Expression::ConstantVariable(_, _, Some(contract_no), var_no) => {
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!(
                                "cannot assign to constant ‘{}’",
                                ns.contracts[*contract_no].variables[*var_no].name
                            ),
                        ));
                        return Err(());
                    }
                    Expression::ConstantVariable(_, _, None, var_no) => {
                        diagnostics.push(Diagnostic::error(
                            *loc,
                            format!("cannot assign to constant ‘{}’", ns.constants[*var_no].name),
                        ));
                        return Err(());
                    }
                    Expression::StorageVariable(_, _, var_contract_no, var_no) => {
                        let store_var = &ns.contracts[*var_contract_no].variables[*var_no];

                        if store_var.immutable && !ns.functions[function_no].is_constructor() {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                format!(
                                    "cannot assign to immutable ‘{}’ outside of constructor",
                                    store_var.name
                                ),
                            ));
                            return Err(());
                        }
                    }
                    Expression::Variable(_, _, _) => (),
                    _ => match e.ty() {
                        Type::Ref(_) | Type::StorageRef(false, _) => (),
                        _ => {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                "expression is not assignable".to_string(),
                            ));
                            return Err(());
                        }
                    },
                }

                assigned_variable(ns, &e, symtable);
                left_tys.push(Some(e.ty()));
                fields.push(DestructureField::Expression(e));
            }
            Some(pt::Parameter {
                loc,
                ty,
                storage,
                name: Some(name),
            }) => {
                let (ty, ty_loc) =
                    resolve_var_decl_ty(ty, storage, file_no, contract_no, ns, diagnostics)?;

                if let Some(pos) = symtable.add(
                    name,
                    ty.clone(),
                    ns,
                    None,
                    VariableUsage::DestructureVariable,
                    storage.clone(),
                ) {
                    ns.check_shadowing(file_no, contract_no, name);

                    left_tys.push(Some(ty.clone()));

                    fields.push(DestructureField::VariableDecl(
                        pos,
                        Parameter {
                            loc: *loc,
                            name: name.name.to_owned(),
                            name_loc: Some(name.loc),
                            ty,
                            ty_loc,
                            indexed: false,
                        },
                    ));
                }
            }
        }
    }

    let expr = destructure_values(
        loc,
        expr,
        &left_tys,
        &fields,
        file_no,
        contract_no,
        function_no,
        unchecked,
        symtable,
        ns,
        diagnostics,
    )?;

    Ok(Statement::Destructure(*loc, fields, expr))
}

fn destructure_values(
    loc: &pt::Loc,
    expr: &pt::Expression,
    left_tys: &[Option<Type>],
    fields: &[DestructureField],
    file_no: usize,
    contract_no: Option<usize>,
    function_no: usize,
    unchecked: bool,
    symtable: &mut Symtable,
    ns: &mut Namespace,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let expr = match expr {
        pt::Expression::FunctionCall(loc, ty, args) => {
            let res = function_call_expr(
                loc,
                ty,
                args,
                file_no,
                contract_no,
                Some(function_no),
                ns,
                symtable,
                false,
                unchecked,
                diagnostics,
            )?;
            check_function_call(ns, &res, symtable);
            res
        }
        pt::Expression::NamedFunctionCall(loc, ty, args) => {
            let res = named_function_call_expr(
                loc,
                ty,
                args,
                file_no,
                contract_no,
                Some(function_no),
                unchecked,
                ns,
                symtable,
                diagnostics,
            )?;
            check_function_call(ns, &res, symtable);
            res
        }
        pt::Expression::Ternary(loc, cond, left, right) => {
            let cond = expression(
                cond,
                file_no,
                contract_no,
                Some(function_no),
                ns,
                symtable,
                false,
                unchecked,
                diagnostics,
                Some(&Type::Bool),
            )?;

            used_variable(ns, &cond, symtable);
            let left = destructure_values(
                &left.loc(),
                left,
                left_tys,
                fields,
                file_no,
                contract_no,
                function_no,
                unchecked,
                symtable,
                ns,
                diagnostics,
            )?;
            used_variable(ns, &left, symtable);
            let right = destructure_values(
                &right.loc(),
                right,
                left_tys,
                fields,
                file_no,
                contract_no,
                function_no,
                unchecked,
                symtable,
                ns,
                diagnostics,
            )?;
            used_variable(ns, &right, symtable);

            return Ok(Expression::Ternary(
                *loc,
                Type::Unreachable,
                Box::new(cond),
                Box::new(left),
                Box::new(right),
            ));
        }
        _ => {
            let mut list = Vec::new();

            let exprs = parameter_list_to_expr_list(expr, diagnostics)?;

            if exprs.len() != left_tys.len() {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "destructuring assignment has {} elements on the left and {} on the right",
                        left_tys.len(),
                        exprs.len(),
                    ),
                ));
                return Err(());
            }

            for (i, e) in exprs.iter().enumerate() {
                let e = expression(
                    e,
                    file_no,
                    contract_no,
                    Some(function_no),
                    ns,
                    symtable,
                    false,
                    unchecked,
                    diagnostics,
                    left_tys[i].as_ref(),
                )?;

                match e.ty() {
                    Type::Void | Type::Unreachable => {
                        diagnostics.push(Diagnostic::error(
                            e.loc(),
                            "function does not return a value".to_string(),
                        ));
                        return Err(());
                    }
                    _ => {
                        used_variable(ns, &e, symtable);
                    }
                }

                list.push(e);
            }

            Expression::List(*loc, list)
        }
    };

    let mut right_tys = expr.tys();

    // Return type void or unreachable are synthetic
    if right_tys.len() == 1 && (right_tys[0] == Type::Unreachable || right_tys[0] == Type::Void) {
        right_tys.truncate(0);
    }

    if left_tys.len() != right_tys.len() {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "destructuring assignment has {} elements on the left and {} on the right",
                left_tys.len(),
                right_tys.len()
            ),
        ));
        return Err(());
    }

    // check that the values can be cast
    for (i, field) in fields.iter().enumerate() {
        if let Some(left_ty) = &left_tys[i] {
            let loc = field.loc().unwrap();
            let _ = cast(
                &loc,
                Expression::FunctionArg(loc, right_tys[i].clone(), i),
                left_ty,
                true,
                ns,
                diagnostics,
            )?;
        }
    }

    Ok(expr)
}

/// Resolve the type of a variable declaration
fn resolve_var_decl_ty(
    ty: &pt::Expression,
    storage: &Option<pt::StorageLocation>,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(Type, pt::Loc), ()> {
    let mut loc_ty = ty.loc();
    let mut var_ty = ns.resolve_type(file_no, contract_no, false, ty, diagnostics)?;

    if let Some(storage) = storage {
        if !var_ty.can_have_data_location() {
            diagnostics.push(Diagnostic::error(
                *storage.loc(),
                format!(
                    "data location ‘{}’ only allowed for array, struct or mapping type",
                    storage
                ),
            ));
            return Err(());
        }

        if let pt::StorageLocation::Storage(loc) = storage {
            loc_ty.2 = loc.2;
            var_ty = Type::StorageRef(false, Box::new(var_ty));
        }

        // Note we are completely ignoring memory or calldata data locations. Everything
        // will be stored in memory.
    }

    if var_ty.contains_mapping(ns) && !var_ty.is_contract_storage() {
        diagnostics.push(Diagnostic::error(
            ty.loc(),
            "mapping only allowed in storage".to_string(),
        ));
        return Err(());
    }

    if !var_ty.is_contract_storage() && !var_ty.fits_in_memory(ns) {
        diagnostics.push(Diagnostic::error(
            ty.loc(),
            "type is too large to fit into memory".to_string(),
        ));
        return Err(());
    }

    Ok((var_ty, loc_ty))
}

/// Parse return statement with values
fn return_with_values(
    returns: &pt::Expression,
    loc: &pt::Loc,
    file_no: usize,
    contract_no: Option<usize>,
    function_no: usize,
    unchecked: bool,
    symtable: &mut Symtable,
    ns: &mut Namespace,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Vec<Expression>, ()> {
    let returns = parameter_list_to_expr_list(returns, diagnostics)?;

    let no_returns = ns.functions[function_no].returns.len();

    if no_returns > 0 && returns.is_empty() {
        diagnostics.push(Diagnostic::error(
            *loc,
            format!(
                "missing return value, {} return values expected",
                no_returns
            ),
        ));
        return Err(());
    }

    if no_returns == 0 && !returns.is_empty() {
        diagnostics.push(Diagnostic::error(
            *loc,
            "function has no return values".to_string(),
        ));
        return Err(());
    }

    if no_returns != returns.len() {
        diagnostics.push(Diagnostic::error(
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
        let ty = ns.functions[function_no].returns[i].ty.clone();

        let e = expression(
            r,
            file_no,
            contract_no,
            Some(function_no),
            ns,
            symtable,
            false,
            unchecked,
            diagnostics,
            Some(&ty),
        )?;

        exprs.push(cast(&r.loc(), e, &ty, true, ns, diagnostics)?);
    }

    Ok(exprs)
}

/// The parser generates parameter lists for lists. Sometimes this needs to be a
/// simple expression list.
pub fn parameter_list_to_expr_list<'a>(
    e: &'a pt::Expression,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Vec<&'a pt::Expression>, ()> {
    if let pt::Expression::List(_, v) = &e {
        let mut list = Vec::new();
        let mut broken = false;

        for e in v {
            match &e.1 {
                None => {
                    diagnostics.push(Diagnostic::error(e.0, "stray comma".to_string()));
                    broken = true;
                }
                Some(pt::Parameter {
                    name: Some(name), ..
                }) => {
                    diagnostics.push(Diagnostic::error(
                        name.loc,
                        "single value expected".to_string(),
                    ));
                    broken = true;
                }
                Some(pt::Parameter {
                    storage: Some(storage),
                    ..
                }) => {
                    diagnostics.push(Diagnostic::error(
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
#[allow(clippy::too_many_arguments)]
fn try_catch(
    loc: &pt::Loc,
    expr: &pt::Expression,
    returns_and_ok: &Option<(Vec<(pt::Loc, Option<pt::Parameter>)>, Box<pt::Statement>)>,
    error_stmt: &Option<Box<(pt::Identifier, pt::Parameter, pt::Statement)>>,
    catch_stmt: &(pt::Parameter, pt::Statement),
    file_no: usize,
    contract_no: Option<usize>,
    function_no: usize,
    unchecked: bool,
    symtable: &mut Symtable,
    loops: &mut LoopScopes,
    ns: &mut Namespace,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(Statement, bool), ()> {
    let mut expr = expr;
    let mut ok = None;

    while let pt::Expression::FunctionCallBlock(_, e, block) = expr {
        if ok.is_some() {
            diagnostics.push(Diagnostic::error(
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
            let res = function_call_expr(
                loc,
                ty,
                args,
                file_no,
                contract_no,
                Some(function_no),
                ns,
                symtable,
                false,
                unchecked,
                diagnostics,
            )?;
            check_function_call(ns, &res, symtable);
            res
        }
        pt::Expression::NamedFunctionCall(loc, ty, args) => {
            let res = named_function_call_expr(
                loc,
                ty,
                args,
                file_no,
                contract_no,
                Some(function_no),
                unchecked,
                ns,
                symtable,
                diagnostics,
            )?;

            check_function_call(ns, &res, symtable);

            res
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
                    let res = new(
                        loc,
                        ty,
                        args,
                        file_no,
                        contract_no,
                        Some(function_no),
                        unchecked,
                        ns,
                        symtable,
                        diagnostics,
                    )?;
                    check_function_call(ns, &res, symtable);

                    res
                }
                pt::Expression::NamedFunctionCall(_, ty, args) => {
                    let res = constructor_named_args(
                        loc,
                        ty,
                        args,
                        file_no,
                        contract_no,
                        Some(function_no),
                        unchecked,
                        ns,
                        symtable,
                        diagnostics,
                    )?;
                    check_function_call(ns, &res, symtable);

                    res
                }
                _ => unreachable!(),
            }
        }
        _ => {
            diagnostics.push(Diagnostic::error(
                expr.loc(),
                "try only supports external calls or constructor calls".to_string(),
            ));
            return Err(());
        }
    };

    let mut returns = &Vec::new();

    if let Some((rets, block)) = returns_and_ok {
        if ok.is_some() {
            diagnostics.push(Diagnostic::error(
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

            diagnostics.push(Diagnostic::error(
                pt::Loc(file_no, pos, pos),
                "code block missing for no catch".to_string(),
            ));
            return Err(());
        }
    };

    symtable.new_scope();

    let mut args = match &fcall {
        Expression::ExternalFunctionCall {
            returns: func_returns,
            ..
        } => {
            let mut func_returns = func_returns.clone();

            if func_returns == vec![Type::Void] {
                func_returns = vec![];
            }

            if returns.len() != func_returns.len() {
                diagnostics.push(Diagnostic::error(
                    expr.loc(),
                    format!(
                        "try returns list has {} entries while function returns {} values",
                        returns.len(),
                        func_returns.len()
                    ),
                ));
                return Err(());
            }

            func_returns
        }
        Expression::Constructor { contract_no, .. } => match returns.len() {
            0 => Vec::new(),
            1 => vec![Type::Contract(*contract_no)],
            _ => {
                diagnostics.push(Diagnostic::error(
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
            diagnostics.push(Diagnostic::error(
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
                let (ret_ty, ty_loc) =
                    resolve_var_decl_ty(ty, storage, file_no, contract_no, ns, diagnostics)?;

                if arg_ty != ret_ty {
                    diagnostics.push(Diagnostic::error(
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
                    if let Some(pos) = symtable.add(
                        name,
                        ret_ty.clone(),
                        ns,
                        None,
                        VariableUsage::TryCatchReturns,
                        storage.clone(),
                    ) {
                        ns.check_shadowing(file_no, contract_no, name);
                        params.push((
                            Some(pos),
                            Parameter {
                                loc: param.0,
                                ty: ret_ty,
                                ty_loc,
                                name: name.name.to_string(),
                                name_loc: Some(name.loc),
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
                            ty_loc,
                            indexed: false,
                            name: "".to_string(),
                            name_loc: None,
                        },
                    ));
                }
            }
            None => {
                diagnostics.push(Diagnostic::error(
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
        ok,
        &mut ok_resolved,
        file_no,
        contract_no,
        function_no,
        unchecked,
        symtable,
        loops,
        ns,
        diagnostics,
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

        let (error_ty, ty_loc) = resolve_var_decl_ty(
            &error_stmt.1.ty,
            &error_stmt.1.storage,
            file_no,
            contract_no,
            ns,
            diagnostics,
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
            ty_loc,
            name: "".to_string(),
            name_loc: None,
            indexed: false,
        };

        if let Some(name) = &error_stmt.1.name {
            if let Some(pos) = symtable.add(
                name,
                Type::String,
                ns,
                None,
                VariableUsage::TryCatchErrorString,
                error_stmt.1.storage.clone(),
            ) {
                ns.check_shadowing(file_no, contract_no, name);

                error_pos = Some(pos);
                error_param.name = name.name.to_string();
                error_param.name_loc = Some(name.loc);
            }
        }

        let reachable = statement(
            &error_stmt.2,
            &mut error_stmt_resolved,
            file_no,
            contract_no,
            function_no,
            unchecked,
            symtable,
            loops,
            ns,
            diagnostics,
        )?;

        finally_reachable |= reachable;

        symtable.leave_scope();

        Some((error_pos, error_param, error_stmt_resolved))
    } else {
        None
    };

    let (catch_ty, ty_loc) = resolve_var_decl_ty(
        &catch_stmt.0.ty,
        &catch_stmt.0.storage,
        file_no,
        contract_no,
        ns,
        diagnostics,
    )?;

    if catch_ty != Type::DynamicBytes {
        diagnostics.push(Diagnostic::error(
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
        ty_loc,
        name: "".to_owned(),
        name_loc: None,
        indexed: false,
    };
    let mut catch_param_pos = None;
    let mut catch_stmt_resolved = Vec::new();

    if let Some(name) = &catch_stmt.0.name {
        if let Some(pos) = symtable.add(
            name,
            catch_ty,
            ns,
            None,
            VariableUsage::TryCatchErrorBytes,
            catch_stmt.0.storage.clone(),
        ) {
            ns.check_shadowing(file_no, contract_no, name);
            catch_param_pos = Some(pos);
            catch_param.name = name.name.to_string();
            catch_param.name_loc = Some(name.loc);
        }
    }

    let reachable = statement(
        &catch_stmt.1,
        &mut catch_stmt_resolved,
        file_no,
        contract_no,
        function_no,
        unchecked,
        symtable,
        loops,
        ns,
        diagnostics,
    )?;

    finally_reachable |= reachable;

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
