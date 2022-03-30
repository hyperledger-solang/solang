use crate::ast::Namespace;
use crate::sema::assembly::ast::{AssemblyExpression, AssemblyStatement};
use crate::sema::assembly::block::resolve_assembly_block;
use crate::sema::assembly::builtin::{assembly_unsupported_builtin, parse_builtin_keyword};
use crate::sema::assembly::expression::{
    check_type, resolve_assembly_expression, resolve_function_call,
};
use crate::sema::assembly::for_loop::resolve_for_loop;
use crate::sema::assembly::functions::FunctionsTable;
use crate::sema::assembly::switch::{resolve_condition, resolve_switch};
use crate::sema::assembly::types::get_default_type_from_identifier;
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{LoopScopes, Symtable, VariableInitializer, VariableUsage};
use solang_parser::diagnostics::{ErrorType, Level, Note};
use solang_parser::pt::AssemblyTypedIdentifier;
use solang_parser::{pt, Diagnostic};

/// Resolves an assembly statement. Returns a boolean that indicates if the next statement is reachable.
pub(crate) fn resolve_assembly_statement(
    statement: &pt::AssemblyStatement,
    context: &ExprContext,
    reachable: bool,
    loop_scope: &mut LoopScopes,
    symtable: &mut Symtable,
    resolved_statements: &mut Vec<(AssemblyStatement, bool)>,
    function_table: &mut FunctionsTable,
    ns: &mut Namespace,
) -> Result<bool, ()> {
    match statement {
        pt::AssemblyStatement::FunctionDefinition(_) => Ok(true),
        pt::AssemblyStatement::FunctionCall(func_call) => {
            let data =
                resolve_top_level_function_call(func_call, function_table, context, symtable, ns)?;
            resolved_statements.push((data.0, reachable));
            Ok(data.1)
        }

        pt::AssemblyStatement::Block(block) => {
            let data = resolve_assembly_block(
                &block.loc,
                &block.statements,
                context,
                reachable,
                loop_scope,
                function_table,
                symtable,
                ns,
            );
            resolved_statements.push((AssemblyStatement::Block(Box::new(data.0)), reachable));
            Ok(data.1)
        }

        pt::AssemblyStatement::VariableDeclaration(loc, variables, initializer) => {
            resolved_statements.push((
                resolve_variable_declaration(
                    loc,
                    variables,
                    initializer,
                    function_table,
                    context,
                    symtable,
                    ns,
                )?,
                reachable,
            ));
            Ok(true)
        }

        pt::AssemblyStatement::Assign(loc, lhs, rhs) => {
            resolved_statements.push((
                resolve_assignment(loc, lhs, rhs, context, function_table, symtable, ns)?,
                reachable,
            ));
            Ok(true)
        }

        pt::AssemblyStatement::If(loc, condition, body) => {
            resolved_statements.push((
                resolve_if_block(
                    loc,
                    condition,
                    &body.statements,
                    context,
                    reachable,
                    loop_scope,
                    function_table,
                    symtable,
                    ns,
                )?,
                reachable,
            ));
            Ok(true)
        }

        pt::AssemblyStatement::Switch(switch_statement) => {
            let resolved_switch = resolve_switch(
                switch_statement,
                context,
                reachable,
                function_table,
                loop_scope,
                symtable,
                ns,
            )?;
            resolved_statements.push((resolved_switch.0, reachable));
            Ok(resolved_switch.1)
        }

        pt::AssemblyStatement::Break(loc) => {
            if loop_scope.do_break() {
                resolved_statements.push((AssemblyStatement::Break(*loc), reachable));
                Ok(false)
            } else {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "break statement outside a for loop".to_string(),
                ));
                Err(())
            }
        }

        pt::AssemblyStatement::Continue(loc) => {
            if loop_scope.do_continue() {
                resolved_statements.push((AssemblyStatement::Continue(*loc), reachable));
                Ok(false)
            } else {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "continue statement outside a for loop".to_string(),
                ));
                Err(())
            }
        }

        pt::AssemblyStatement::Leave(loc) => {
            if !context.yul_function {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "leave statement cannot be used outside a function".to_string(),
                ));
                return Err(());
            }
            resolved_statements.push((AssemblyStatement::Leave(*loc), reachable));
            Ok(false)
        }

        pt::AssemblyStatement::For(for_statement) => {
            let resolved_for = resolve_for_loop(
                for_statement,
                context,
                reachable,
                loop_scope,
                symtable,
                function_table,
                ns,
            )?;
            resolved_statements.push((resolved_for.0, reachable));
            Ok(resolved_for.1)
        }
    }
}

/// Top-leve function calls must not return anything, so there is a special function to handle them.
fn resolve_top_level_function_call(
    func_call: &pt::AssemblyFunctionCall,
    function_table: &mut FunctionsTable,
    context: &ExprContext,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<(AssemblyStatement, bool), ()> {
    match resolve_function_call(function_table, func_call, context, symtable, ns) {
        Ok(AssemblyExpression::BuiltInCall(loc, ty, args)) => {
            let func_prototype = ty.get_prototype_info();
            if func_prototype.no_returns != 0 {
                ns.diagnostics.push(Diagnostic::error(
                    loc,
                    "top level function calls must not return anything".to_string(),
                ));
                return Err(());
            }
            Ok((
                AssemblyStatement::BuiltInCall(loc, ty, args),
                !func_prototype.stops_execution,
            ))
        }
        Ok(AssemblyExpression::FunctionCall(loc, function_no, args)) => {
            let func = function_table.get(function_no).unwrap();
            if !func.returns.is_empty() {
                ns.diagnostics.push(Diagnostic::error(
                    loc,
                    "top level function calls must not return anything".to_string(),
                ));
                return Err(());
            }
            Ok((
                AssemblyStatement::FunctionCall(loc, function_no, args),
                true,
            ))
        }

        Ok(_) => {
            unreachable!("sema::assembly::resolve_function_call can only return resolved calls")
        }

        Err(_) => Err(()),
    }
}

fn resolve_variable_declaration(
    loc: &pt::Loc,
    variables: &[AssemblyTypedIdentifier],
    initializer: &Option<pt::AssemblyExpression>,
    function_table: &mut FunctionsTable,
    context: &ExprContext,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<AssemblyStatement, ()> {
    let mut added_variables: Vec<usize> = Vec::with_capacity(variables.len());
    for item in variables {
        if let Some(func) = function_table.find(&item.id.name) {
            ns.diagnostics.push(Diagnostic {
                level: Level::Error,
                ty: ErrorType::DeclarationError,
                pos: item.loc,
                message: format!("name '{}' has been defined as a function", item.id.name),
                notes: vec![Note {
                    pos: func.id.loc,
                    message: "function defined here".to_string(),
                }],
            });
            return Err(());
        } else if assembly_unsupported_builtin(&item.id.name)
            || parse_builtin_keyword(&item.id.name).is_some()
        {
            ns.diagnostics.push(Diagnostic::error(
                item.loc,
                format!(
                    "'{}' is a built-in function and cannot be a variable name",
                    item.id.name
                ),
            ));
            return Err(());
        } else if item.id.name.starts_with("verbatim") {
            ns.diagnostics.push(Diagnostic::error(
                item.loc,
                "the prefix 'verbatim' is reserved for verbatim functions".to_string(),
            ));
            return Err(());
        }

        let ty = get_default_type_from_identifier(&item.ty, ns)?;

        if let Some(pos) = symtable.exclusive_add(
            &item.id,
            ty,
            ns,
            VariableInitializer::Assembly(initializer.is_some()),
            VariableUsage::AssemblyLocalVariable,
            None,
        ) {
            added_variables.push(pos);
        } else {
            return Err(());
        }
    }

    let resolved_init = if let Some(init_expr) = &initializer {
        let resolved_expr =
            resolve_assembly_expression(init_expr, context, symtable, function_table, ns)?;
        check_assignment_compatibility(
            loc,
            variables,
            &resolved_expr,
            context,
            function_table,
            symtable,
            ns,
        );
        Some(resolved_expr)
    } else {
        None
    };

    Ok(AssemblyStatement::VariableDeclaration(
        *loc,
        added_variables,
        resolved_init,
    ))
}

fn resolve_assignment(
    loc: &pt::Loc,
    lhs: &[pt::AssemblyExpression],
    rhs: &pt::AssemblyExpression,
    context: &ExprContext,
    function_table: &mut FunctionsTable,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<AssemblyStatement, ()> {
    let mut resolved_lhs: Vec<AssemblyExpression> = Vec::with_capacity(lhs.len());
    let mut local_ctx = context.clone();
    local_ctx.lvalue = true;
    for item in lhs {
        let resolved = resolve_assembly_expression(item, &local_ctx, symtable, function_table, ns)?;
        if let Some(diagnostic) = check_type(&resolved, &local_ctx, ns, symtable) {
            ns.diagnostics.push(diagnostic);
            return Err(());
        }
        resolved_lhs.push(resolved);
    }

    local_ctx.lvalue = false;
    let resolved_rhs = resolve_assembly_expression(rhs, &local_ctx, symtable, function_table, ns)?;
    check_assignment_compatibility(
        loc,
        &resolved_lhs,
        &resolved_rhs,
        context,
        function_table,
        symtable,
        ns,
    );

    Ok(AssemblyStatement::Assignment(
        *loc,
        resolved_lhs,
        resolved_rhs,
    ))
}

/// Checks the the left hand side of an assignment is compatible with it right hand side
fn check_assignment_compatibility<T>(
    loc: &pt::Loc,
    lhs: &[T],
    rhs: &AssemblyExpression,
    context: &ExprContext,
    function_table: &FunctionsTable,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) {
    match rhs {
        AssemblyExpression::FunctionCall(_, function_no, ..) => {
            let func = function_table.get(*function_no).unwrap();
            if func.returns.len() != lhs.len() {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "{} variables on the left hand side, but the function returns {} values",
                        lhs.len(),
                        func.returns.len()
                    ),
                ));
            }
        }

        AssemblyExpression::BuiltInCall(_, ty, _) => {
            let prototype = ty.get_prototype_info();
            if prototype.no_returns as usize != lhs.len() {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    format!(
                        "{} variables on the left hand side, but the function returns {} values",
                        lhs.len(),
                        prototype.no_returns
                    ),
                ));
            }
        }

        _ => {
            if lhs.len() != 1 {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "a single value cannot be assigned to multiple variables".to_string(),
                ));
            } else if let Some(diagnostic) = check_type(rhs, context, ns, symtable) {
                ns.diagnostics.push(diagnostic);
            }
        }
    }
}

fn resolve_if_block(
    loc: &pt::Loc,
    condition: &pt::AssemblyExpression,
    if_block: &[pt::AssemblyStatement],
    context: &ExprContext,
    reachable: bool,
    loop_scope: &mut LoopScopes,
    function_table: &mut FunctionsTable,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<AssemblyStatement, ()> {
    let resolved_condition = resolve_condition(condition, context, symtable, function_table, ns)?;

    let resolved_block = resolve_assembly_block(
        loc,
        if_block,
        context,
        reachable,
        loop_scope,
        function_table,
        symtable,
        ns,
    );

    Ok(AssemblyStatement::IfBlock(
        *loc,
        resolved_condition,
        Box::new(resolved_block.0),
    ))
}
