use crate::ast::EventDecl;
use crate::sema::ast::{Builtin, CallArgs, Diagnostic, Expression, Namespace};
use crate::sema::symtable::{Symtable, VariableUsage};
use crate::sema::{ast, symtable};
use solang_parser::pt::{ContractTy, Loc};

/// Mark variables as assigned, either in the symbol table (for local variables) or in the
/// Namespace (for storage variables)
pub fn assigned_variable(ns: &mut Namespace, exp: &Expression, symtable: &mut Symtable) {
    match &exp {
        Expression::StorageVariable(_, _, contract_no, offset) => {
            ns.contracts[*contract_no].variables[*offset].assigned = true;
        }

        Expression::Variable(_, _, offset) => {
            let var = symtable.vars.get_mut(offset).unwrap();
            (*var).assigned = true;
        }

        Expression::StructMember(_, _, str, _) => {
            assigned_variable(ns, str, symtable);
        }

        Expression::Subscript(_, _, _, array, index) => {
            assigned_variable(ns, array, symtable);
            used_variable(ns, index, symtable);
        }

        Expression::StorageLoad(_, _, expr)
        | Expression::Load(_, _, expr)
        | Expression::Trunc(_, _, expr)
        | Expression::Cast(_, _, expr)
        | Expression::BytesCast(_, _, _, expr) => {
            assigned_variable(ns, expr, symtable);
        }

        _ => {}
    }
}

/// Mark variables as used, either in the symbol table (for local variables) or in the
/// Namespace (for global constants and storage variables)
/// The functions handles complex expressions in a recursive fashion, such as array length call,
/// assign expressions and array subscripts.
pub fn used_variable(ns: &mut Namespace, exp: &Expression, symtable: &mut Symtable) {
    match &exp {
        Expression::StorageVariable(_, _, contract_no, offset) => {
            ns.contracts[*contract_no].variables[*offset].read = true;
        }

        Expression::Variable(_, _, offset) => {
            let var = symtable.vars.get_mut(offset).unwrap();
            (*var).read = true;
        }

        Expression::ConstantVariable(_, _, Some(contract_no), offset) => {
            ns.contracts[*contract_no].variables[*offset].read = true;
        }

        Expression::ConstantVariable(_, _, None, offset) => {
            ns.constants[*offset].read = true;
        }

        Expression::StructMember(_, _, str, _) => {
            used_variable(ns, str, symtable);
        }

        Expression::Subscript(_, _, _, array, index) => {
            used_variable(ns, array, symtable);
            used_variable(ns, index, symtable);
        }

        Expression::Builtin(_, _, Builtin::ArrayLength, args) => {
            //We should not eliminate an array from the code when 'length' is called
            //So the variable is also assigned
            assigned_variable(ns, &args[0], symtable);
            used_variable(ns, &args[0], symtable);
        }
        Expression::StorageArrayLength {
            loc: _,
            ty: _,
            array,
            ..
        } => {
            //We should not eliminate an array from the code when 'length' is called
            //So the variable is also assigned
            assigned_variable(ns, array, symtable);
            used_variable(ns, array, symtable);
        }

        Expression::StorageLoad(_, _, expr)
        | Expression::Load(_, _, expr)
        | Expression::SignExt(_, _, expr)
        | Expression::ZeroExt(_, _, expr)
        | Expression::Trunc(_, _, expr)
        | Expression::Cast(_, _, expr)
        | Expression::BytesCast(_, _, _, expr) => {
            used_variable(ns, expr, symtable);
        }

        Expression::InternalFunctionCall { .. } | Expression::ExternalFunctionCall { .. } => {
            check_function_call(ns, exp, symtable);
        }

        _ => {}
    }
}

/// Mark function arguments as used. If the function is an attribute of another variable, mark the
/// usage of the latter as well
pub fn check_function_call(ns: &mut Namespace, exp: &Expression, symtable: &mut Symtable) {
    match &exp {
        Expression::Load(..) | Expression::StorageLoad(..) | Expression::Variable(..) => {
            used_variable(ns, exp, symtable);
        }

        Expression::InternalFunctionCall {
            loc: _,
            returns: _,
            function,
            args,
        } => {
            for arg in args {
                used_variable(ns, arg, symtable);
            }
            check_function_call(ns, function, symtable);
        }

        Expression::ExternalFunctionCall {
            loc: _,
            returns: _,
            function,
            args,
            call_args,
        } => {
            for arg in args {
                used_variable(ns, arg, symtable);
            }
            check_call_args(ns, call_args, symtable);
            check_function_call(ns, function, symtable);
        }

        Expression::Constructor {
            loc: _,
            contract_no: _,
            constructor_no: _,
            args,
            call_args,
        } => {
            for arg in args {
                used_variable(ns, arg, symtable);
            }
            check_call_args(ns, call_args, symtable);
        }

        Expression::ExternalFunctionCallRaw {
            loc: _,
            ty: _,
            address,
            args,
            call_args,
        } => {
            used_variable(ns, args, symtable);
            used_variable(ns, address, symtable);
            check_call_args(ns, call_args, symtable);
        }

        Expression::ExternalFunction { address, .. } => {
            used_variable(ns, address, symtable);
        }

        Expression::Builtin(_, _, expr_type, args) => match expr_type {
            Builtin::ArrayPush => {
                assigned_variable(ns, &args[0], symtable);
                if args.len() > 1 {
                    used_variable(ns, &args[1], symtable);
                }
            }

            _ => {
                for arg in args {
                    used_variable(ns, arg, symtable);
                }
            }
        },

        Expression::FormatString(_, args) => {
            for (_, expr) in args {
                used_variable(ns, expr, symtable);
            }
        }

        _ => {}
    }
}

/// Mark function call arguments as used
fn check_call_args(ns: &mut Namespace, call_args: &CallArgs, symtable: &mut Symtable) {
    if let Some(gas) = &call_args.gas {
        used_variable(ns, gas.as_ref(), symtable);
    }
    if let Some(salt) = &call_args.salt {
        used_variable(ns, salt.as_ref(), symtable);
    }
    if let Some(value) = &call_args.value {
        used_variable(ns, value.as_ref(), symtable);
    }
    if let Some(space) = &call_args.space {
        used_variable(ns, space.as_ref(), symtable);
    }
    if let Some(accounts) = &call_args.accounts {
        used_variable(ns, accounts.as_ref(), symtable);
    }
}

/// Marks as used variables that appear in an expression with right and left hand side.
pub fn check_var_usage_expression(
    ns: &mut Namespace,
    left: &Expression,
    right: &Expression,
    symtable: &mut Symtable,
) {
    used_variable(ns, left, symtable);
    used_variable(ns, right, symtable);
}

/// Emit different warning types according to the function variable usage
pub fn emit_warning_local_variable(variable: &symtable::Variable) -> Option<Diagnostic> {
    match &variable.usage_type {
        VariableUsage::Parameter => {
            if !variable.read {
                return Some(Diagnostic::warning(
                    variable.id.loc,
                    format!(
                        "function parameter '{}' has never been read",
                        variable.id.name
                    ),
                ));
            }
            None
        }

        VariableUsage::ReturnVariable => {
            if !variable.assigned {
                return Some(Diagnostic::warning(
                    variable.id.loc,
                    format!(
                        "return variable '{}' has never been assigned",
                        variable.id.name
                    ),
                ));
            }
            None
        }

        VariableUsage::LocalVariable => {
            let assigned = variable.initializer.has_initializer() || variable.assigned;
            if !assigned && !variable.read {
                return Some(Diagnostic::warning(
                    variable.id.loc,
                    format!(
                        "local variable '{}' has never been read nor assigned",
                        variable.id.name
                    ),
                ));
            } else if assigned && !variable.read && !variable.is_reference() {
                // Values assigned to variables that reference others change the value of its reference
                // No warning needed in this case
                return Some(Diagnostic::warning(
                    variable.id.loc,
                    format!(
                        "local variable '{}' has been assigned, but never read",
                        variable.id.name
                    ),
                ));
            }
            None
        }

        VariableUsage::DestructureVariable => {
            if !variable.read {
                return Some(Diagnostic::warning(
                    variable.id.loc,
                    format!(
                        "destructure variable '{}' has never been used",
                        variable.id.name
                    ),
                ));
            }

            None
        }

        VariableUsage::TryCatchReturns => {
            if !variable.read {
                return Some(Diagnostic::warning(
                    variable.id.loc,
                    format!(
                        "try-catch returns variable '{}' has never been read",
                        variable.id.name
                    ),
                ));
            }

            None
        }

        VariableUsage::TryCatchErrorBytes => {
            if !variable.read {
                return Some(Diagnostic::warning(
                    variable.id.loc,
                    format!(
                        "try-catch error bytes '{}' has never been used",
                        variable.id.name
                    ),
                ));
            }

            None
        }

        VariableUsage::TryCatchErrorString => {
            if !variable.read {
                return Some(Diagnostic::warning(
                    variable.id.loc,
                    format!(
                        "try-catch error string '{}' has never been used",
                        variable.id.name
                    ),
                ));
            }

            None
        }
        VariableUsage::YulLocalVariable => {
            let has_value = variable.assigned || variable.initializer.has_initializer();
            if !variable.read && !has_value {
                return Some(Diagnostic::warning(
                    variable.id.loc,
                    format!(
                        "yul variable '{}' has never been read or assigned",
                        variable.id.name
                    ),
                ));
            } else if !variable.read {
                return Some(Diagnostic::warning(
                    variable.id.loc,
                    format!("yul variable '{}' has never been read", variable.id.name),
                ));
            }
            None
        }
        VariableUsage::AnonymousReturnVariable => None,
    }
}

/// Emit warnings depending on the storage variable usage
fn emit_warning_contract_variables(variable: &ast::Variable) -> Option<Diagnostic> {
    if variable.assigned && !variable.read {
        return Some(Diagnostic::warning(
            variable.loc,
            format!(
                "storage variable '{}' has been assigned, but never read",
                variable.name
            ),
        ));
    } else if !variable.assigned && !variable.read {
        return Some(Diagnostic::warning(
            variable.loc,
            format!("storage variable '{}' has never been used", variable.name),
        ));
    }

    //Solidity attributes zero value to contract values that have never been assigned
    //There is no need to raise warning if we use them, as they have a valid value.

    None
}

/// Check for unused constants and storage variables
pub fn check_unused_namespace_variables(ns: &mut Namespace) {
    for contract in &ns.contracts {
        for variable in &contract.variables {
            if let Some(warning) = emit_warning_contract_variables(variable) {
                ns.diagnostics.push(warning);
            }
        }
    }

    // Global constants should have been initialized during declaration
    for constant in &ns.constants {
        if !constant.read {
            ns.diagnostics.push(Diagnostic::warning(
                constant.loc,
                format!("global constant '{}' has never been used", constant.name),
            ));
        }
    }
}

/// Find shadowing events
fn shadowing_events(
    event_no: usize,
    event: &EventDecl,
    shadows: &mut Vec<usize>,
    events: &[(Loc, usize)],
    ns: &Namespace,
) {
    for e in events {
        let other_no = e.1;
        if event_no != other_no && ns.events[other_no].signature == event.signature {
            shadows.push(other_no);
        }
    }
}

/// Check for unused events
pub fn check_unused_events(ns: &mut Namespace) {
    // first we need to calculate which event shadows which
    // an event can be declare on the global scope and re-declared in a contract,
    // and then again redeclared in as base contract. In this case all of the events
    // should be marked as used
    for event_no in 0..ns.events.len() {
        let event = &ns.events[event_no];

        if !event.used {
            continue;
        }

        let mut shadows = Vec::new();

        if let Some(contract_no) = event.contract {
            // is there a global event with the same name
            if let Some(ast::Symbol::Event(events)) =
                ns.variable_symbols
                    .get(&(event.loc.file_no(), None, event.name.to_owned()))
            {
                shadowing_events(event_no, event, &mut shadows, events, ns);
            }

            // is there a base contract with the same name
            for base_no in ns.contract_bases(contract_no) {
                let base_file_no = ns.contracts[base_no].loc.file_no();

                if let Some(ast::Symbol::Event(events)) =
                    ns.variable_symbols
                        .get(&(base_file_no, Some(base_no), event.name.to_owned()))
                {
                    shadowing_events(event_no, event, &mut shadows, events, ns);
                }
            }
        }

        for shadow in shadows {
            ns.events[shadow].used = true;
        }
    }

    for event in &ns.events {
        if !event.used {
            if let Some(contract_no) = event.contract {
                // don't complain about events in interfaces or abstract contracts
                if matches!(
                    ns.contracts[contract_no].ty,
                    ContractTy::Interface(_) | ContractTy::Abstract(_)
                ) {
                    continue;
                }
            }

            ns.diagnostics.push(Diagnostic::warning(
                event.loc,
                format!("event '{}' has never been emitted", event.name),
            ));
        }
    }
}
