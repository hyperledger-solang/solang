// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::Namespace;
use crate::sema::symtable::Symtable;
use crate::sema::yul::ast::YulExpression;

pub(crate) fn assigned_variable(ns: &mut Namespace, exp: &YulExpression, symtable: &mut Symtable) {
    match exp {
        // Considering that semantic analysis already considered the assignment valid
        YulExpression::SolidityLocalVariable(_, _, _, var_no)
        | YulExpression::YulLocalVariable(_, _, var_no) => {
            let var = symtable.vars.get_mut(var_no).unwrap();
            var.assigned = true;
        }

        YulExpression::StorageVariable(_, _, contract_no, var_no) => {
            ns.contracts[*contract_no].variables[*var_no].assigned = true;
        }

        YulExpression::SuffixAccess(_, member, _) => {
            assigned_variable(ns, member, symtable);
        }

        _ => (),
    }
}

pub(crate) fn used_variable(ns: &mut Namespace, exp: &YulExpression, symtable: &mut Symtable) {
    match exp {
        YulExpression::SolidityLocalVariable(_, _, _, var_no)
        | YulExpression::YulLocalVariable(_, _, var_no) => {
            let var = symtable.vars.get_mut(var_no).unwrap();
            var.read = true;
        }

        YulExpression::StorageVariable(_, _, contract_no, var_no) => {
            ns.contracts[*contract_no].variables[*var_no].read = true;
        }

        YulExpression::SuffixAccess(_, member, _) => {
            used_variable(ns, member, symtable);
        }

        _ => (),
    }
}
