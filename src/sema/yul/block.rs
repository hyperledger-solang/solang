// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::Namespace;
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{LoopScopes, Symtable};
use crate::sema::yul::ast::{YulBlock, YulStatement};
use crate::sema::yul::functions::{
    process_function_header, resolve_function_definition, FunctionsTable,
};
use crate::sema::yul::statements::resolve_yul_statement;
use solang_parser::{
    diagnostics::Diagnostic,
    pt::{self, CodeLocation},
};

/// Resolve an yul block.
/// Returns the resolved block and a boolean that tells us if the next statement is reachable.
pub fn resolve_yul_block(
    loc: &pt::Loc,
    statements: &[pt::YulStatement],
    context: &mut ExprContext,
    reachable: bool,
    loop_scope: &mut LoopScopes,
    function_table: &mut FunctionsTable,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> (YulBlock, bool) {
    function_table.enter_scope();
    context.enter_scope();

    let (body, mut next_reachable) = process_statements(
        statements,
        context,
        reachable,
        symtable,
        loop_scope,
        function_table,
        ns,
    );

    next_reachable &= reachable;
    context.leave_scope(symtable, *loc);
    function_table.leave_scope(ns);

    (
        YulBlock {
            loc: *loc,
            reachable,
            next_reachable,
            statements: body,
        },
        next_reachable,
    )
}

/// Resolves an array of yul statements.
/// Returns a vector of tuples (resolved_statement, reachable) and a boolean that tells us if the
/// next statement is reachable
pub(crate) fn process_statements(
    statements: &[pt::YulStatement],
    context: &mut ExprContext,
    mut reachable: bool,
    symtable: &mut Symtable,
    loop_scope: &mut LoopScopes,
    functions_table: &mut FunctionsTable,
    ns: &mut Namespace,
) -> (Vec<YulStatement>, bool) {
    let mut func_count: usize = 0;
    for item in statements {
        if let pt::YulStatement::FunctionDefinition(fun_def) = item {
            process_function_header(fun_def, functions_table, ns);
            func_count += 1;
        }
    }

    for item in statements {
        if let pt::YulStatement::FunctionDefinition(func_def) = item {
            // If an error was generate while processing the function header, it is not added to the
            // functions table, so we do not resolve its body.
            let index = if let Some(index) = functions_table.function_index(&func_def.id.name) {
                index
            } else {
                continue;
            };

            if let Ok(resolved_func) =
                resolve_function_definition(func_def, functions_table, context, ns)
            {
                functions_table.resolved_functions[index] = resolved_func;
            }
        }
    }

    let mut body: Vec<YulStatement> = Vec::with_capacity(statements.len() - func_count);
    let mut has_unreachable = false;
    for item in statements {
        match resolve_yul_statement(
            item,
            context,
            reachable,
            loop_scope,
            symtable,
            &mut body,
            functions_table,
            ns,
        ) {
            Ok(can_reach_next_statement) => {
                /* There shouldn't be warnings of unreachable statements for function definitions.
                    let x := foo(1, 2)
                    return(x, 2)
                    function foo(a, b) -> ret {
                        ret := add(a, b)
                    }
                The function definition is not unreachable, because it does not execute anything.
                */
                if !reachable
                    && !has_unreachable
                    && !matches!(item, pt::YulStatement::FunctionDefinition(..))
                {
                    ns.diagnostics.push(Diagnostic::warning(
                        item.loc(),
                        "unreachable yul statement".to_string(),
                    ));
                    has_unreachable = true;
                }
                reachable &= can_reach_next_statement;
            }
            Err(_) => {
                break;
            }
        }
    }

    (body, reachable)
}
