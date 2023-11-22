// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::Namespace;
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
    memory_safe: bool,
    statements: &[pt::YulStatement],
    context: &mut ExprContext,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> (InlineAssembly, bool) {
    let start = ns.yul_functions.len();
    let mut functions_table = FunctionsTable::new(start);
    functions_table.enter_scope();
    context.enter_scope();
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

    context.leave_scope(symtable, *loc);
    functions_table.leave_scope(ns);
    let end = start + functions_table.resolved_functions.len();
    ns.yul_functions
        .append(&mut functions_table.resolved_functions);

    (
        InlineAssembly {
            loc: *loc,
            memory_safe,
            body,
            functions: std::ops::Range { start, end },
        },
        reachable,
    )
}
