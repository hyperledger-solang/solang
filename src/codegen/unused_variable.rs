use crate::codegen::cfg::{ControlFlowGraph, Vartable};
use crate::parser::pt;
use crate::sema::ast::{Expression, Function, Namespace};
use crate::sema::symtable::VariableUsage;

/// This struct saves the parameters to call 'check_side_effects_expressions'
/// using 'expression.recurse'
pub struct SideEffectsCheckParameters<'a> {
    pub cfg: &'a mut ControlFlowGraph,
    pub contract_no: usize,
    pub func: Option<&'a Function>,
    pub ns: &'a Namespace,
    pub vartab: &'a mut Vartable,
}

/// Check if we should remove an assignment. The expression in the argument is the left-hand side
/// of the assignment
pub fn should_remove_assignment(ns: &Namespace, exp: &Expression, func: &Function) -> bool {
    match &exp {
        Expression::StorageVariable(_, _, contract_no, offset) => {
            let var = &ns.contracts[*contract_no].variables[*offset];

            !var.read
        }

        Expression::Variable(_, _, offset) => should_remove_variable(
            offset,
            func,
            func.symtable.vars.get(offset).unwrap().initializer.as_ref(),
        ),

        Expression::StructMember(_, _, str, _) => should_remove_assignment(ns, str, func),

        Expression::Subscript(_, _, array, _)
        | Expression::DynamicArraySubscript(_, _, array, _)
        | Expression::StorageBytesSubscript(_, array, _) => {
            should_remove_assignment(ns, array, func)
        }

        Expression::StorageLoad(_, _, expr)
        | Expression::Load(_, _, expr)
        | Expression::Trunc(_, _, expr)
        | Expression::Cast(_, _, expr)
        | Expression::BytesCast(_, _, _, expr) => should_remove_assignment(ns, expr, func),

        _ => false,
    }
}

/// Checks if we should remove a variable
pub fn should_remove_variable(
    pos: &usize,
    func: &Function,
    initializer: Option<&Expression>,
) -> bool {
    let var = &func.symtable.vars[pos];

    //If the variable has never been read nor assigned, we can remove it right away.
    if !var.read && !var.assigned {
        return true;
    }

    // If the variable has been assigned, we must detect special cases
    // Parameters and return variable cannot be removed
    if !var.read
        && var.assigned
        && matches!(
            var.usage_type,
            VariableUsage::DestructureVariable | VariableUsage::LocalVariable
        )
    {
        // If the variable has the memory or storage keyword, it can be a reference to another variable.
        // In this case, an assigment may change the value of the variable it is referencing.
        if !matches!(
            var.storage_location,
            Some(pt::StorageLocation::Memory(_)) | Some(pt::StorageLocation::Storage(_))
        ) {
            return true;
        } else if let Some(expr) = initializer {
            // We can only remove variable with memory and storage keywords if the initializer is
            // an array allocation, a constructor and a struct literal (only available in the local scope),
            return matches!(
                expr,
                Expression::AllocDynamicArray(..)
                    | Expression::ArrayLiteral(..)
                    | Expression::Constructor { .. }
                    | Expression::StructLiteral(..)
            );
        }
    }

    false
}
