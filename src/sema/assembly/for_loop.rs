use crate::ast::Namespace;
use crate::sema::assembly::block::{process_statements, resolve_assembly_block, AssemblyBlock};
use crate::sema::assembly::expression::resolve_assembly_expression;
use crate::sema::assembly::functions::FunctionsTable;
use crate::sema::assembly::statements::AssemblyStatement;
use crate::sema::assembly::types::verify_type_from_expression;
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{LoopScopes, Symtable};
use solang_parser::{pt, Diagnostic};

/// Resolve a for-loop statement
/// Returns the resolved block and a bool to indicate if the next statement is reachable.
pub(crate) fn resolve_for_loop(
    assembly_for: &pt::AssemblyFor,
    context: &ExprContext,
    mut reachable: bool,
    loop_scope: &mut LoopScopes,
    symtable: &mut Symtable,
    function_table: &mut FunctionsTable,
    ns: &mut Namespace,
) -> Result<(AssemblyStatement, bool), ()> {
    symtable.new_scope();
    function_table.new_scope();

    let resolved_init_block = resolve_for_init_block(
        &assembly_for.init_block,
        context,
        loop_scope,
        symtable,
        function_table,
        ns,
    )?;
    reachable &= resolved_init_block.1;

    let resolved_cond = resolve_assembly_expression(
        &assembly_for.condition,
        context,
        symtable,
        function_table,
        ns,
    )?;
    match verify_type_from_expression(&resolved_cond, function_table) {
        Ok(_) => (),
        Err(diagnostic) => {
            ns.diagnostics.push(diagnostic);
            return Err(());
        }
    }

    loop_scope.new_scope();

    let resolved_exec_block = resolve_assembly_block(
        &assembly_for.execution_block.loc,
        &assembly_for.execution_block.statements,
        context,
        reachable,
        loop_scope,
        function_table,
        symtable,
        ns,
    );
    reachable &= resolved_exec_block.1;

    loop_scope.leave_scope();

    let resolved_post_block = resolve_assembly_block(
        &assembly_for.post_block.loc,
        &assembly_for.post_block.statements,
        context,
        reachable,
        loop_scope,
        function_table,
        symtable,
        ns,
    );

    symtable.leave_scope();
    function_table.leave_scope();

    Ok((
        AssemblyStatement::For {
            loc: assembly_for.loc,
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
    init_block: &pt::AssemblyBlock,
    context: &ExprContext,
    loop_scope: &mut LoopScopes,
    symtable: &mut Symtable,
    function_table: &mut FunctionsTable,
    ns: &mut Namespace,
) -> Result<(AssemblyBlock, bool), ()> {
    for item in &init_block.statements {
        if matches!(item, pt::AssemblyStatement::FunctionDefinition(_)) {
            ns.diagnostics.push(Diagnostic::error(
                item.loc(),
                "function definitions are not allowed inside for-init block".to_string(),
            ));
            return Err(());
        }
    }

    let (body, reachable) = process_statements(
        &init_block.statements,
        context,
        true,
        symtable,
        loop_scope,
        function_table,
        ns,
    );

    Ok((
        AssemblyBlock {
            loc: init_block.loc,
            body,
        },
        reachable,
    ))
}
