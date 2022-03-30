use crate::ast::Namespace;
use crate::sema::assembly::ast::{AssemblyBlock, AssemblyStatement};
use crate::sema::assembly::functions::{
    process_function_header, resolve_function_definition, FunctionsTable,
};
use crate::sema::assembly::statements::resolve_assembly_statement;
use crate::sema::expression::ExprContext;
use crate::sema::symtable::{LoopScopes, Symtable};
use solang_parser::{pt, Diagnostic};

/// Resolve an assembly block.
/// Returns the resolved block and a boolean that tells us if the next statement is reachable.
pub fn resolve_assembly_block(
    loc: &pt::Loc,
    statements: &[pt::AssemblyStatement],
    context: &ExprContext,
    mut reachable: bool,
    loop_scope: &mut LoopScopes,
    function_table: &mut FunctionsTable,
    symtable: &mut Symtable,
    ns: &mut Namespace,
) -> (AssemblyBlock, bool) {
    function_table.new_scope();
    symtable.new_scope();

    let (body, local_reachable) = process_statements(
        statements,
        context,
        reachable,
        symtable,
        loop_scope,
        function_table,
        ns,
    );

    reachable &= local_reachable;

    symtable.leave_scope();
    function_table.leave_scope(ns);

    (AssemblyBlock { loc: *loc, body }, reachable)
}

/// Resolves an array of assembly statements.
/// Returns a vector of tuples (resolved_statement, reachable) and a boolean that tells us if the
/// next statement is reachable
pub(crate) fn process_statements(
    statements: &[pt::AssemblyStatement],
    context: &ExprContext,
    mut reachable: bool,
    symtable: &mut Symtable,
    loop_scope: &mut LoopScopes,
    functions_table: &mut FunctionsTable,
    ns: &mut Namespace,
) -> (Vec<(AssemblyStatement, bool)>, bool) {
    let mut func_count: usize = 0;
    for item in statements {
        if let pt::AssemblyStatement::FunctionDefinition(fun_def) = item {
            process_function_header(fun_def, functions_table, ns);
            func_count += 1;
        }
    }

    for item in statements {
        if let pt::AssemblyStatement::FunctionDefinition(func_def) = item {
            if let Ok(resolved_func) =
                resolve_function_definition(func_def, functions_table, context, ns)
            {
                functions_table.resolved_functions.push(resolved_func);
            }
        }
    }

    let mut body: Vec<(AssemblyStatement, bool)> =
        Vec::with_capacity(statements.len() - func_count);
    let mut has_unreachable = false;
    for item in statements {
        match resolve_assembly_statement(
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
                    && !matches!(item, pt::AssemblyStatement::FunctionDefinition(..))
                {
                    ns.diagnostics.push(Diagnostic::warning(
                        item.loc(),
                        "unreachable assembly statement".to_string(),
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
