use crate::codegen::cfg::{ControlFlowGraph, Vartable};
use crate::parser::pt;
use crate::sema::ast::{Expression, Function, Namespace};
use crate::sema::symtable::VariableUsage;

pub struct SideEffectsCheckParameters<'a> {
    pub cfg: &'a mut ControlFlowGraph,
    pub contract_no: usize,
    pub func: Option<&'a Function>,
    pub ns: &'a Namespace,
    pub vartab: &'a mut Vartable,
}

pub fn should_remove_assignment(ns: &Namespace, exp: &Expression, func: &Function) -> bool {
    match &exp {
        Expression::StorageVariable(_, _, contract_no, offset) => {
            let var = &ns.contracts[*contract_no].variables[*offset];
            if matches!(
                var.visibility,
                pt::Visibility::Public(_) | pt::Visibility::External(_)
            ) {
                return false;
            }

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

        _ => true,
    }
}

pub fn should_remove_variable(
    pos: &usize,
    func: &Function,
    initializer: Option<&Expression>,
) -> bool {
    let var = func.symtable.vars.get(pos).unwrap();
    if !var.read && !var.assigned {
        return true;
    }

    if !var.read
        && var.assigned
        && matches!(
            var.usage_type,
            VariableUsage::DestructureVariable | VariableUsage::LocalVariable
        )
    {
        if !matches!(
            var.storage_location,
            Some(pt::StorageLocation::Memory(_)) | Some(pt::StorageLocation::Storage(_))
        ) {
            return true;
        } else if let Some(expr) = initializer {
            return match expr {
                Expression::AllocDynamicArray(..)
                | Expression::Constructor { .. }
                | Expression::StructLiteral(..) => true,

                _ => false,
            };
        }
    }

    false
}
