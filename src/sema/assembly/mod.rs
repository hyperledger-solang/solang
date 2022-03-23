use crate::ast::Namespace;
use crate::sema::assembly::block::process_statements;
use crate::sema::assembly::functions::{AssemblyFunction, FunctionsTable};
use crate::sema::assembly::statements::AssemblyStatement;
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{LoopScopes, Symtable};
use solang_parser::pt;

mod block;
mod builtin;
mod expression;
mod for_loop;
mod functions;
mod statements;
mod switch;
mod tests;
mod types;

// TODO: Mark functions and variables as used/unused

#[derive(Debug, Clone)]
pub struct InlineAssembly {
    pub loc: pt::Loc,
    pub body: Vec<(AssemblyStatement, bool)>,
    pub functions: Vec<AssemblyFunction>,
}

/// Resolves a block of inline assembly
/// Returns the resolved block and a bool to indicate if the next statement is reachable.
pub fn resolve_inline_assembly(
    loc: &pt::Loc,
    statements: &[pt::AssemblyStatement],
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
    functions_table.leave_scope();

    (
        InlineAssembly {
            loc: *loc,
            body,
            functions: functions_table.resolved_functions,
        },
        reachable,
    )
}
