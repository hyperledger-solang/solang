// SPDX-License-Identifier: Apache-2.0

use super::Options;
use crate::codegen::{cfg::ControlFlowGraph, vartable::Vartable, OptimizationLevel};
use crate::sema::ast::RetrieveType;
use crate::sema::ast::{Builtin, Expression, Function, Namespace};
use crate::sema::symtable::VariableUsage;

/// This struct saves the parameters to call 'check_side_effects_expressions'
/// using 'expression.recurse'
pub struct SideEffectsCheckParameters<'a> {
    pub cfg: &'a mut ControlFlowGraph,
    pub contract_no: usize,
    pub func: Option<&'a Function>,
    pub ns: &'a Namespace,
    pub vartab: &'a mut Vartable,
    pub opt: &'a Options,
}

/// Check if we should remove an assignment. The expression in the argument is the left-hand side
/// of the assignment
pub fn should_remove_assignment(
    exp: &Expression,
    func: &Function,
    opt: &Options,
    ns: &Namespace,
) -> bool {
    if opt.opt_level == OptimizationLevel::None {
        return false;
    }

    match &exp {
        Expression::Variable { var_no, .. } => should_remove_variable(*var_no, func, opt, ns),

        Expression::StructMember { expr, .. } => should_remove_assignment(expr, func, opt, ns),

        Expression::Subscript { array, .. } => should_remove_assignment(array, func, opt, ns),

        Expression::StorageLoad { expr, .. }
        | Expression::Load { expr, .. }
        | Expression::Trunc { expr, .. }
        | Expression::Cast { expr, .. }
        | Expression::BytesCast { expr, .. } => should_remove_assignment(expr, func, opt, ns),

        Expression::Builtin {
            kind: Builtin::ArrayLength,
            args,
            ..
        } => should_remove_assignment(&args[0], func, opt, ns),

        Expression::Builtin {
            kind: Builtin::ArrayPop | Builtin::ArrayPush,
            args,
            ..
        } => {
            // If the argument is a storage reference, the operation shall not be removed
            if args[0].ty().is_contract_storage() {
                return false;
            }

            should_remove_assignment(&args[0], func, opt, ns)
        }

        _ => false,
    }
}

/// Checks if we should remove a variable
pub fn should_remove_variable(pos: usize, func: &Function, opt: &Options, ns: &Namespace) -> bool {
    if opt.opt_level == OptimizationLevel::None {
        return false;
    }

    let var = &func.symtable.vars[&pos];

    //If the variable has never been read nor assigned, we can remove it right away.
    if !var.read && !var.assigned {
        return true;
    }

    // If the variable has been assigned, we must detect special cases
    // Parameters and return variables cannot be removed
    if !var.read
        && var.assigned
        && matches!(
            var.usage_type,
            VariableUsage::DestructureVariable | VariableUsage::LocalVariable
        )
    {
        // Variables that are reference to other cannot be removed
        return !var.is_reference(ns);
    }

    false
}

// TODO: unused variables should remove Yul assignments!
