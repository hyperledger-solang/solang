// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::Namespace;
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{LoopScopes, Symtable};
use crate::sema::yul::ast::{YulBlock, YulStatement};
use crate::sema::yul::block::{process_statements, resolve_yul_block};
use crate::sema::yul::functions::FunctionsTable;
use crate::sema::yul::switch::resolve_condition;
use solang_parser::{
    diagnostics::Diagnostic,
    pt::{self, CodeLocation},
};

/// Resolve a for-loop statement
/// Returns the resolved block and a bool to indicate if the next statement is reachable.
pub(crate) fn resolve_for_loop(
    yul_for: &pt::YulFor,
    context: &mut ExprContext,
    reachable: bool,
    loop_scope: &mut LoopScopes,
    symtable: &mut Symtable,
    function_table: &mut FunctionsTable,
    ns: &mut Namespace,
) -> Result<(YulStatement, bool), ()> {
    context.enter_scope();
    function_table.enter_scope();
    let mut next_reachable = reachable;
    let resolved_init_block = resolve_for_init_block(
        &yul_for.init_block,
        context,
        next_reachable,
        loop_scope,
        symtable,
        function_table,
        ns,
    )?;
    next_reachable &= resolved_init_block.1;

    let resolved_cond =
        resolve_condition(&yul_for.condition, context, symtable, function_table, ns)?;

    loop_scope.enter_scope();

    let resolved_exec_block = resolve_yul_block(
        &yul_for.execution_block.loc,
        &yul_for.execution_block.statements,
        context,
        next_reachable,
        loop_scope,
        function_table,
        symtable,
        ns,
    );
    next_reachable &= resolved_exec_block.1;

    loop_scope.leave_scope();

    let resolved_post_block = resolve_yul_block(
        &yul_for.post_block.loc,
        &yul_for.post_block.statements,
        context,
        next_reachable,
        loop_scope,
        function_table,
        symtable,
        ns,
    );

    context.leave_scope(symtable, yul_for.loc);
    function_table.leave_scope(ns);

    Ok((
        YulStatement::For {
            loc: yul_for.loc,
            reachable,
            init_block: resolved_init_block.0,
            condition: resolved_cond,
            post_block: resolved_post_block.0,
            execution_block: resolved_exec_block.0,
        },
        resolved_init_block.1,
    ))
}

/// Resolve for initialization block.
/// Returns the resolved block and a bool to indicate if the next statement is reachable.
fn resolve_for_init_block(
    init_block: &pt::YulBlock,
    context: &mut ExprContext,
    reachable: bool,
    loop_scope: &mut LoopScopes,
    symtable: &mut Symtable,
    function_table: &mut FunctionsTable,
    ns: &mut Namespace,
) -> Result<(YulBlock, bool), ()> {
    for item in &init_block.statements {
        if matches!(item, pt::YulStatement::FunctionDefinition(_)) {
            ns.diagnostics.push(Diagnostic::error(
                item.loc(),
                "function definitions are not allowed inside for-init block".to_string(),
            ));
            return Err(());
        }
    }

    let (body, next_reachable) = process_statements(
        &init_block.statements,
        context,
        reachable,
        symtable,
        loop_scope,
        function_table,
        ns,
    );

    Ok((
        YulBlock {
            loc: init_block.loc,
            reachable,
            next_reachable,
            statements: body,
        },
        next_reachable,
    ))
}
