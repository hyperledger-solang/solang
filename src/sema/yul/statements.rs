// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{Namespace, Type};
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{LoopScopes, Symtable, VariableInitializer, VariableUsage};
use crate::sema::yul::ast::{YulExpression, YulStatement};
use crate::sema::yul::block::resolve_yul_block;
use crate::sema::yul::builtin::{parse_builtin_keyword, yul_unsupported_builtin};
use crate::sema::yul::expression::{check_type, resolve_function_call, resolve_yul_expression};
use crate::sema::yul::for_loop::resolve_for_loop;
use crate::sema::yul::functions::FunctionsTable;
use crate::sema::yul::switch::{resolve_condition, resolve_switch};
use crate::sema::yul::types::get_default_type_from_identifier;
use solang_parser::diagnostics::{ErrorType, Level, Note};
use solang_parser::pt::YulTypedIdentifier;
use solang_parser::{diagnostics::Diagnostic, pt};

/// Resolves an yul statement. Returns a boolean that indicates if the next statement is reachable.
pub(crate) fn resolve_yul_statement(
    statement: &pt::YulStatement,
    context: &mut ExprContext,
    reachable: bool,
    loop_scope: &mut LoopScopes,
    symtable: &mut Symtable,
    resolved_statements: &mut Vec<YulStatement>,
    function_table: &mut FunctionsTable,
    ns: &mut Namespace,
) -> Result<bool, ()> {
    match statement {
        pt::YulStatement::FunctionDefinition(_) => Ok(true),
        pt::YulStatement::FunctionCall(func_call) => {
            let data = resolve_top_level_function_call(
                func_call,
                reachable,
                function_table,
                context,
                symtable,
                ns,
            )?;
            resolved_statements.push(data.0);
            Ok(data.1)
        }

        pt::YulStatement::Block(block) => {
            let data = resolve_yul_block(
                &block.loc,
                &block.statements,
                context,
                reachable,
                loop_scope,
                function_table,
                symtable,
                ns,
            );
            resolved_statements.push(YulStatement::Block(Box::new(data.0)));
            Ok(data.1)
        }

        pt::YulStatement::VariableDeclaration(loc, variables, initializer) => {
            resolved_statements.push(resolve_variable_declaration(
                loc,
                variables,
                initializer,
                reachable,
                function_table,
                context,
                symtable,
                ns,
            )?);
            Ok(true)
        }

        pt::YulStatement::Assign(loc, lhs, rhs) => {
            resolved_statements.push(resolve_assignment(
                loc,
                lhs,
                rhs,
                context,
                reachable,
                function_table,
                symtable,
                ns,
            )?);
            Ok(true)
        }

        pt::YulStatement::If(loc, condition, body) => {
            resolved_statements.push(resolve_if_block(
                loc,
                condition,
                &body.statements,
                context,
                reachable,
                loop_scope,
                function_table,
                symtable,
                ns,
            )?);
            Ok(true)
        }

        pt::YulStatement::Switch(switch_statement) => {
            let resolved_switch = resolve_switch(
                switch_statement,
                context,
                reachable,
                function_table,
                loop_scope,
                symtable,
                ns,
            )?;
            resolved_statements.push(resolved_switch.0);
            Ok(resolved_switch.1)
        }

        pt::YulStatement::Break(loc) => {
            if loop_scope.do_break() {
                resolved_statements.push(YulStatement::Break(*loc, reachable));
                Ok(false)
            } else {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "break statement outside a for loop".to_string(),
                ));
                Err(())
            }
        }

        pt::YulStatement::Continue(loc) => {
            if loop_scope.do_continue() {
                resolved_statements.push(YulStatement::Continue(*loc, reachable));
                Ok(false)
            } else {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "continue statement outside a for loop".to_string(),
                ));
                Err(())
            }
        }

        pt::YulStatement::Leave(loc) => {
            if !context.yul_function {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "leave statement cannot be used outside a function".to_string(),
                ));
                return Err(());
            }
            resolved_statements.push(YulStatement::Leave(*loc, reachable));
            Ok(false)
        }

        pt::YulStatement::For(for_statement) => {
            let resolved_for = resolve_for_loop(
                for_statement,
                context,
                reachable,
                loop_scope,
                symtable,
                function_table,
                ns,
            )?;
            resolved_statements.push(resolved_for.0);
            Ok(resolved_for.1)
        }
        pt::YulStatement::Error(..) => Err(()),
    }
}

/// Top-leve function calls must not return anything, so there is a special function to handle them.
fn resolve_top_level_function_call(
    func_call: &pt::YulFunctionCall,
    reachable: bool,
    function_table: &mut FunctionsTable,
    context: &mut ExprContext,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<(YulStatement, bool), ()> {
    match resolve_function_call(function_table, func_call, context, symtable, ns) {
        Ok(YulExpression::BuiltInCall(loc, ty, args)) => {
            let func_prototype = ty.get_prototype_info();
            if func_prototype.no_returns != 0 {
                ns.diagnostics.push(Diagnostic::error(
                    loc,
                    "top level function calls must not return anything".to_string(),
                ));
                return Err(());
            }
            Ok((
                YulStatement::BuiltInCall(loc, reachable, ty, args),
                !func_prototype.stops_execution,
            ))
        }
        Ok(YulExpression::FunctionCall(loc, function_no, args, returns)) => {
            if !returns.is_empty() {
                ns.diagnostics.push(Diagnostic::error(
                    loc,
                    "top level function calls must not return anything".to_string(),
                ));
                return Err(());
            }
            Ok((
                YulStatement::FunctionCall(loc, reachable, function_no, args),
                true,
            ))
        }

        Ok(_) => {
            unreachable!("sema::yul::resolve_function_call can only return resolved calls")
        }

        Err(_) => Err(()),
    }
}

fn resolve_variable_declaration(
    loc: &pt::Loc,
    variables: &[YulTypedIdentifier],
    initializer: &Option<pt::YulExpression>,
    reachable: bool,
    function_table: &mut FunctionsTable,
    context: &mut ExprContext,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<YulStatement, ()> {
    let mut added_variables: Vec<(usize, Type)> = Vec::with_capacity(variables.len());
    for item in variables {
        if let Some(func) = function_table.find(&item.id.name) {
            ns.diagnostics.push(Diagnostic {
                level: Level::Error,
                ty: ErrorType::DeclarationError,
                loc: item.loc,
                message: format!("name '{}' has been defined as a function", item.id.name),
                notes: vec![Note {
                    loc: func.id.loc,
                    message: "function defined here".to_string(),
                }],
            });
            return Err(());
        } else if yul_unsupported_builtin(&item.id.name)
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
            ty.clone(),
            ns,
            VariableInitializer::Yul(initializer.is_some()),
            VariableUsage::YulLocalVariable,
            None,
            context,
        ) {
            added_variables.push((pos, ty));
        } else {
            return Err(());
        }
    }

    let resolved_init = if let Some(init_expr) = &initializer {
        let resolved_expr =
            resolve_yul_expression(init_expr, context, symtable, function_table, ns)?;
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

    Ok(YulStatement::VariableDeclaration(
        *loc,
        reachable,
        added_variables,
        resolved_init,
    ))
}

fn resolve_assignment(
    loc: &pt::Loc,
    lhs: &[pt::YulExpression],
    rhs: &pt::YulExpression,
    context: &mut ExprContext,
    reachable: bool,
    function_table: &mut FunctionsTable,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<YulStatement, ()> {
    let mut resolved_lhs: Vec<YulExpression> = Vec::with_capacity(lhs.len());
    let prev_lvalue = context.lvalue;
    context.lvalue = true;

    let mut context = scopeguard::guard(context, |context| {
        context.lvalue = prev_lvalue;
    });

    for item in lhs {
        let resolved = resolve_yul_expression(item, &mut context, symtable, function_table, ns)?;
        if let Some(diagnostic) = check_type(&resolved, &mut context, ns, symtable) {
            ns.diagnostics.push(diagnostic);
            return Err(());
        }
        resolved_lhs.push(resolved);
    }

    context.lvalue = false;
    let resolved_rhs = resolve_yul_expression(rhs, &mut context, symtable, function_table, ns)?;
    check_assignment_compatibility(
        loc,
        &resolved_lhs,
        &resolved_rhs,
        &mut context,
        function_table,
        symtable,
        ns,
    );

    Ok(YulStatement::Assignment(
        *loc,
        reachable,
        resolved_lhs,
        resolved_rhs,
    ))
}

/// Checks the the left hand side of an assignment is compatible with it right hand side
fn check_assignment_compatibility<T>(
    loc: &pt::Loc,
    lhs: &[T],
    rhs: &YulExpression,
    context: &mut ExprContext,
    function_table: &FunctionsTable,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) {
    match rhs {
        YulExpression::FunctionCall(_, function_no, ..) => {
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

        YulExpression::BuiltInCall(_, ty, _) => {
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
    condition: &pt::YulExpression,
    if_block: &[pt::YulStatement],
    context: &mut ExprContext,
    reachable: bool,
    loop_scope: &mut LoopScopes,
    function_table: &mut FunctionsTable,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> Result<YulStatement, ()> {
    let resolved_condition = resolve_condition(condition, context, symtable, function_table, ns)?;

    let resolved_block = resolve_yul_block(
        loc,
        if_block,
        context,
        reachable,
        loop_scope,
        function_table,
        symtable,
        ns,
    );

    Ok(YulStatement::IfBlock(
        *loc,
        reachable,
        resolved_condition,
        Box::new(resolved_block.0),
    ))
}
