// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{Builtin, DestructureField, Expression, Namespace, Statement};
use crate::sema::Recurse;
use indexmap::IndexSet;
use solang_parser::pt;

#[derive(Default)]
struct CallList {
    pub solidity: IndexSet<usize>,
    pub yul: IndexSet<usize>,
}

/// After generating the AST for a contract, we should have a list of
/// all the functions a contract calls in `all_functions`. This should
/// include any libraries and global functions
pub fn add_external_functions(contract_no: usize, ns: &mut Namespace) {
    let mut call_list = CallList::default();

    for var in &ns.contracts[contract_no].variables {
        if let Some(init) = &var.initializer {
            init.recurse(&mut call_list, check_expression);
        }
    }

    for function_no in ns.contracts[contract_no].all_functions.keys() {
        let func = &ns.functions[*function_no];

        for stmt in &func.body {
            stmt.recurse(&mut call_list, check_statement);
        }
    }

    // we've now collected all the functions which are called.
    while !call_list.solidity.is_empty() || !call_list.yul.is_empty() {
        let mut new_call_list = CallList::default();

        for function_no in &call_list.solidity {
            let func = &ns.functions[*function_no];

            for stmt in &func.body {
                stmt.recurse(&mut new_call_list, check_statement);
            }
        }

        // add functions to contract functions list
        for function_no in &call_list.solidity {
            if ns.functions[*function_no].loc_prototype != pt::Loc::Builtin {
                let func = &ns.functions[*function_no];

                // make sure we are not adding a public function which is not a base or library
                if func.is_public() {
                    // free function are not public, else this unwrap would panic
                    let func_contract = func.contract_no.unwrap();

                    assert!(
                        ns.contract_bases(contract_no).contains(&func_contract)
                            || ns.contracts[func_contract].is_library()
                    );
                }

                ns.contracts[contract_no]
                    .all_functions
                    .insert(*function_no, usize::MAX);
            }
        }

        for yul_function_no in &call_list.yul {
            ns.contracts[contract_no]
                .yul_functions
                .push(*yul_function_no);
        }

        call_list.solidity.clear();
        call_list.yul.clear();

        for function_no in &new_call_list.solidity {
            if !ns.contracts[contract_no]
                .all_functions
                .contains_key(function_no)
            {
                call_list.solidity.insert(*function_no);
            }
        }

        for yul_func_no in &new_call_list.yul {
            ns.contracts[contract_no].yul_functions.push(*yul_func_no);
        }
    }

    // now that we have the final list of functions, we can populate the list
    // of events this contract emits
    let mut emits_events = Vec::new();

    for function_no in ns.contracts[contract_no].all_functions.keys() {
        let func = &ns.functions[*function_no];

        for event_no in &func.emits_events {
            if !emits_events.contains(event_no) {
                emits_events.push(*event_no);
            }
        }
    }

    ns.contracts[contract_no].emits_events = emits_events;
}

fn check_expression(expr: &Expression, call_list: &mut CallList) -> bool {
    match expr {
        Expression::UserDefinedOperator { function_no, .. }
        | Expression::InternalFunction { function_no, .. } => {
            call_list.solidity.insert(*function_no);
        }
        Expression::Builtin {
            kind: Builtin::AbiEncodeCall,
            args,
            ..
        } => {
            for expr in &args[1..] {
                check_expression(expr, call_list);
            }
            return false;
        }
        Expression::Builtin {
            kind: Builtin::FunctionSelector,
            ..
        } => return false,
        _ => (),
    }

    true
}

fn check_statement(stmt: &Statement, call_list: &mut CallList) -> bool {
    match stmt {
        Statement::VariableDecl(_, _, _, Some(expr)) => {
            expr.recurse(call_list, check_expression);
        }
        Statement::VariableDecl(_, _, _, None) => (),
        Statement::If(_, _, cond, _, _) => {
            cond.recurse(call_list, check_expression);
        }
        Statement::For {
            cond: Some(cond), ..
        } => {
            cond.recurse(call_list, check_expression);
        }
        Statement::For { cond: None, .. } => (),
        Statement::DoWhile(_, _, _, cond) | Statement::While(_, _, cond, _) => {
            cond.recurse(call_list, check_expression);
        }
        Statement::Expression(_, _, expr) => {
            // if expression is a singular Expression::InternalFunction, then does nothing
            // and it's never called.
            if !matches!(expr, Expression::InternalFunction { .. }) {
                expr.recurse(call_list, check_expression);
            }
        }
        Statement::Delete(_, _, expr) => {
            expr.recurse(call_list, check_expression);
        }
        Statement::Destructure(_, fields, expr) => {
            // This is either a list or internal/external function call
            expr.recurse(call_list, check_expression);

            for field in fields {
                if let DestructureField::Expression(expr) = field {
                    expr.recurse(call_list, check_expression);
                }
            }
        }
        Statement::Return(_, expr) => {
            if let Some(e) = expr {
                e.recurse(call_list, check_expression);
            }
        }
        Statement::TryCatch(_, _, try_catch) => {
            try_catch.expr.recurse(call_list, check_expression);
        }
        Statement::Revert { args, .. } | Statement::Emit { args, .. } => {
            for e in args {
                e.recurse(call_list, check_expression);
            }
        }
        Statement::Block { .. }
        | Statement::Break(_)
        | Statement::Continue(_)
        | Statement::Underscore(_) => (),
        Statement::Assembly(inline_assembly, _) => {
            for func_no in inline_assembly.functions.start..inline_assembly.functions.end {
                call_list.yul.insert(func_no);
            }
        }
    }

    true
}
