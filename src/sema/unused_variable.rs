// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::{
    Builtin, CallArgs, Diagnostic, EventDecl, Expression, ExternalCallAccounts, Namespace,
    RetrieveType,
};
use crate::sema::symtable::{Symtable, VariableUsage};
use crate::sema::{ast, symtable};
use solang_parser::pt::{ContractTy, Loc};

/// Mark variables as assigned, either in the symbol table (for local variables) or in the
/// Namespace (for storage variables)
pub fn assigned_variable(ns: &mut Namespace, exp: &Expression, symtable: &mut Symtable) {
    match &exp {
        Expression::StorageVariable {
            contract_no,
            var_no,
            ..
        } => {
            ns.contracts[*contract_no].variables[*var_no].assigned = true;
        }

        Expression::Variable { var_no, .. } => {
            let var = symtable.vars.get_mut(var_no).unwrap();
            var.assigned = true;
        }

        Expression::StructMember { expr, .. } => {
            assigned_variable(ns, expr, symtable);
        }

        Expression::Subscript { array, index, .. } => {
            if array.ty().is_contract_storage() {
                subscript_variable(ns, array, symtable);
            } else {
                assigned_variable(ns, array, symtable);
            }
            used_variable(ns, index, symtable);
        }

        Expression::StorageLoad { expr, .. }
        | Expression::Load { expr, .. }
        | Expression::Trunc { expr, .. }
        | Expression::Cast { expr, .. }
        | Expression::BytesCast { expr, .. } => {
            assigned_variable(ns, expr, symtable);
        }

        _ => {}
    }
}

// We have two cases here
//  contract c {
//      int[2] case1;
//
//      function f(int[2] storage case2) {
//          case1[0] = 1;
//          case2[0] = 1;
//      }
//  }
//  The subscript for case1 is an assignment
//  The subscript for case2 is a read
fn subscript_variable(ns: &mut Namespace, exp: &Expression, symtable: &mut Symtable) {
    match &exp {
        Expression::StorageVariable {
            contract_no,
            var_no,
            ..
        } => {
            ns.contracts[*contract_no].variables[*var_no].assigned = true;
        }

        Expression::Variable { var_no, .. } => {
            let var = symtable.vars.get_mut(var_no).unwrap();
            var.read = true;
        }

        Expression::StructMember { expr, .. } => {
            subscript_variable(ns, expr, symtable);
        }

        Expression::Subscript { array, index, .. } => {
            subscript_variable(ns, array, symtable);
            subscript_variable(ns, index, symtable);
        }

        Expression::InternalFunctionCall { .. } | Expression::ExternalFunctionCall { .. } => {
            check_function_call(ns, exp, symtable);
        }

        _ => (),
    }
}

/// Mark variables as used, either in the symbol table (for local variables) or in the
/// Namespace (for global constants and storage variables)
/// The functions handles complex expressions in a recursive fashion, such as array length call,
/// assign expressions and array subscripts.
pub fn used_variable(ns: &mut Namespace, exp: &Expression, symtable: &mut Symtable) {
    match &exp {
        Expression::StorageVariable {
            contract_no,
            var_no,
            ..
        } => {
            ns.contracts[*contract_no].variables[*var_no].read = true;
        }

        Expression::Variable { var_no, .. } => {
            let var = symtable.vars.get_mut(var_no).unwrap();
            var.read = true;
        }

        Expression::ConstantVariable {
            contract_no: Some(contract_no),
            var_no,
            ..
        } => {
            ns.contracts[*contract_no].variables[*var_no].read = true;
        }

        Expression::ConstantVariable {
            contract_no: None,
            var_no,
            ..
        } => {
            ns.constants[*var_no].read = true;
        }

        Expression::StructMember { expr, .. } => {
            used_variable(ns, expr, symtable);
        }

        Expression::Subscript { array, index, .. } => {
            used_variable(ns, array, symtable);
            used_variable(ns, index, symtable);
        }

        Expression::Builtin {
            kind: Builtin::ArrayLength,
            args,
            ..
        } => {
            used_variable(ns, &args[0], symtable);
        }

        Expression::Builtin {
            kind: Builtin::ArrayPush | Builtin::ArrayPop,
            args,
            ..
        } => {
            // Array push and pop return values, so they are both read and assigned.
            used_variable(ns, &args[0], symtable);
            assigned_variable(ns, &args[0], symtable);
        }

        Expression::StorageArrayLength { array, .. } => {
            // We should not eliminate an array from the code when 'length' is called
            // So the variable is also assigned
            assigned_variable(ns, array, symtable);
            used_variable(ns, array, symtable);
        }

        Expression::StorageLoad { expr, .. }
        | Expression::Load { expr, .. }
        | Expression::SignExt { expr, .. }
        | Expression::ZeroExt { expr, .. }
        | Expression::Trunc { expr, .. }
        | Expression::Cast { expr, .. }
        | Expression::BytesCast { expr, .. } => {
            used_variable(ns, expr, symtable);
        }

        Expression::ExternalFunction { .. }
        | Expression::InternalFunctionCall { .. }
        | Expression::ExternalFunctionCall { .. } => {
            check_function_call(ns, exp, symtable);
        }

        _ => {}
    }
}

/// Mark function arguments as used. If the function is an attribute of another variable, mark the
/// usage of the latter as well
pub fn check_function_call(ns: &mut Namespace, exp: &Expression, symtable: &mut Symtable) {
    match &exp {
        Expression::Load { .. } | Expression::StorageLoad { .. } | Expression::Variable { .. } => {
            used_variable(ns, exp, symtable);
        }

        Expression::InternalFunctionCall { function, args, .. } => {
            for arg in args {
                used_variable(ns, arg, symtable);
            }
            check_function_call(ns, function, symtable);
        }

        Expression::ExternalFunctionCall {
            function,
            args,
            call_args,
            ..
        } => {
            for arg in args {
                used_variable(ns, arg, symtable);
            }
            check_call_args(ns, call_args, symtable);
            check_function_call(ns, function, symtable);
        }

        Expression::Constructor {
            args, call_args, ..
        } => {
            for arg in args {
                used_variable(ns, arg, symtable);
            }
            check_call_args(ns, call_args, symtable);
        }

        Expression::ExternalFunctionCallRaw {
            address,
            args,
            call_args,
            ..
        } => {
            used_variable(ns, args, symtable);
            used_variable(ns, address, symtable);
            check_call_args(ns, call_args, symtable);
        }

        Expression::ExternalFunction { address, .. } => {
            used_variable(ns, address, symtable);
        }

        Expression::Builtin {
            kind: expr_type,
            args,
            ..
        } => match expr_type {
            Builtin::ArrayPush | Builtin::ArrayPop => {
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

        Expression::FormatString { format, .. } => {
            for (_, expr) in format {
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
    if let ExternalCallAccounts::Present(accounts) = &call_args.accounts {
        used_variable(ns, accounts.as_ref(), symtable);
    }
    if let Some(seeds) = &call_args.seeds {
        used_variable(ns, seeds.as_ref(), symtable);
    }
    if let Some(flags) = &call_args.flags {
        used_variable(ns, flags.as_ref(), symtable);
    }
    if let Some(program_id) = &call_args.program_id {
        used_variable(ns, program_id.as_ref(), symtable);
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
pub fn emit_warning_local_variable(
    variable: &symtable::Variable,
    ns: &Namespace,
) -> Option<Diagnostic> {
    match &variable.usage_type {
        VariableUsage::Parameter => {
            if (!variable.read && !variable.ty.is_reference_type(ns))
                || (!variable.read && !variable.assigned && variable.ty.is_reference_type(ns))
            {
                return Some(Diagnostic::warning(
                    variable.id.loc,
                    format!("function parameter '{}' is unused", variable.id.name),
                ));
            }
            None
        }

        VariableUsage::ReturnVariable => {
            if !variable.assigned {
                if variable.ty.is_contract_storage() {
                    return Some(Diagnostic::error(
                        variable.id.loc,
                        format!(
                            "storage reference '{}' must be assigned a value",
                            variable.id.name
                        ),
                    ));
                } else {
                    return Some(Diagnostic::warning(
                        variable.id.loc,
                        format!(
                            "return variable '{}' has never been assigned",
                            variable.id.name
                        ),
                    ));
                }
            }
            None
        }

        VariableUsage::LocalVariable => {
            let assigned = variable.initializer.has_initializer() || variable.assigned;
            if !variable.assigned && !variable.read {
                return Some(Diagnostic::warning(
                    variable.id.loc,
                    format!("local variable '{}' is unused", variable.id.name),
                ));
            } else if assigned && !variable.read && !variable.is_reference(ns) {
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
                    .get(&(event.loc.file_no(), None, event.id.name.to_owned()))
            {
                shadowing_events(event_no, event, &mut shadows, events, ns);
            }

            // is there a base contract with the same name
            for base_no in ns.contract_bases(contract_no) {
                let base_file_no = ns.contracts[base_no].loc.file_no();

                if let Some(ast::Symbol::Event(events)) = ns.variable_symbols.get(&(
                    base_file_no,
                    Some(base_no),
                    event.id.name.to_owned(),
                )) {
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
                event.id.loc,
                format!("event '{}' has never been emitted", event.id),
            ));
        }
    }
}

/// Check for unused error definitions. Here NotEnoughBalance is never used in a
/// revert statement.
///
/// ```ignore
/// contract c {
///     error NotEnoughBalance(address user);
///     error UnknownUser(address user);
///
///     mapping(address => uint64) balances;
///
///     function balance(address user) public returns (uint64 balance) {
///         balance = balances[user];
///         if (balance == 0) {
///             revert UnknownUser(user);
///         }
///     }
/// }
/// ```
pub fn check_unused_errors(ns: &mut Namespace) {
    // it is an error to shadow error definitions
    for error in &ns.errors {
        if !error.used {
            if let Some(contract_no) = error.contract {
                // don't complain about error definitions in interfaces or abstract contracts
                if matches!(
                    ns.contracts[contract_no].ty,
                    ContractTy::Interface(_) | ContractTy::Abstract(_)
                ) {
                    continue;
                }
            }

            ns.diagnostics.push(Diagnostic::warning(
                error.loc,
                format!("error '{}' has never been used", error.name),
            ));
        }
    }
}
