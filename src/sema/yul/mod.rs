use crate::ast::Namespace;
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{LoopScopes, Symtable};
use crate::sema::yul::ast::InlineAssembly;
use crate::sema::yul::block::process_statements;
use crate::sema::yul::functions::FunctionsTable;
use solang_parser::pt;

pub mod ast;
mod block;
pub mod builtin;
mod expression;
mod for_loop;
mod functions;
mod statements;
mod switch;
mod tests;
mod types;
mod unused_variable;

/// Resolves a block of inline assembly
/// Returns the resolved block and a bool to indicate if the next statement is reachable.
pub fn resolve_inline_assembly(
    loc: &pt::Loc,
    statements: &[pt::YulStatement],
    context: &ExprContext,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> (InlineAssembly, bool) {
    let mut functions_table = FunctionsTable::new();
    functions_table.new_scope();
    symtable.new_scope();
    let mut loop_scope = LoopScopes::new();

    let (body, reachable) = process_statements(
        statements,
        context,
        true,
        symtable,
        &mut loop_scope,
        &mut functions_table,
        ns,
    );

    symtable.leave_scope();
    functions_table.leave_scope(ns);

    (
        InlineAssembly {
            loc: *loc,
            body,
            functions: functions_table.resolved_functions,
        },
        reachable,
    )
}
