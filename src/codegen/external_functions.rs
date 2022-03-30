use crate::sema::ast::{DestructureField, Expression, Namespace, Statement};

/// After generating the AST for a contract, we should have a list of
/// all the functions a contract calls in `all_functions`. This should
/// include any libraries and global functions
pub fn add_external_functions(contract_no: usize, ns: &mut Namespace) {
    let mut call_list = Vec::new();

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
    while !call_list.is_empty() {
        let mut new_call_list = Vec::new();

        for function_no in &call_list {
            let func = &ns.functions[*function_no];

            for stmt in &func.body {
                stmt.recurse(&mut new_call_list, check_statement);
            }
        }

        // add functions to contract functions list
        for function_no in &call_list {
            ns.contracts[contract_no]
                .all_functions
                .insert(*function_no, usize::MAX);
        }

        call_list.truncate(0);

        for function_no in new_call_list.into_iter() {
            if !ns.contracts[contract_no]
                .all_functions
                .contains_key(&function_no)
            {
                call_list.push(function_no);
            }
        }
    }

    // now that we have the final list of functions, we can populate the list
    // of events this contract emits
    let mut send_events = Vec::new();

    for function_no in ns.contracts[contract_no].all_functions.keys() {
        let func = &ns.functions[*function_no];

        for event_no in &func.emits_events {
            if !send_events.contains(event_no) {
                send_events.push(*event_no);
            }
        }
    }

    ns.contracts[contract_no].sends_events = send_events;
}

fn check_expression(expr: &Expression, call_list: &mut Vec<usize>) -> bool {
    if let Expression::InternalFunction { function_no, .. } = expr {
        if !call_list.contains(function_no) {
            call_list.push(*function_no);
        }
    }

    true
}

fn check_statement(stmt: &Statement, call_list: &mut Vec<usize>) -> bool {
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
            expr.recurse(call_list, check_expression);
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
        Statement::Return(_, exprs) => {
            for e in exprs {
                e.recurse(call_list, check_expression);
            }
        }
        Statement::TryCatch(_, _, try_catch) => {
            try_catch.expr.recurse(call_list, check_expression);
        }
        Statement::Emit { args, .. } => {
            for e in args {
                e.recurse(call_list, check_expression);
            }
        }
        Statement::Block { .. }
        | Statement::Break(_)
        | Statement::Continue(_)
        | Statement::Underscore(_) => (),
        Statement::Assembly(..) => {
            unimplemented!("Assembly block codegen not yet ready!");
        }
    }

    true
}
