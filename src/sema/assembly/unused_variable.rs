use crate::ast::Namespace;
use crate::sema::assembly::expression::AssemblyExpression;
use crate::sema::symtable::Symtable;

pub(crate) fn assigned_variable(
    ns: &mut Namespace,
    exp: &AssemblyExpression,
    symtable: &mut Symtable,
) {
    match exp {
        // Considering that semantic analysis already considered the assignment valid
        AssemblyExpression::SolidityLocalVariable(_, _, _, var_no)
        | AssemblyExpression::AssemblyLocalVariable(_, _, var_no) => {
            let var = symtable.vars.get_mut(var_no).unwrap();
            (*var).assigned = true;
        }

        AssemblyExpression::StorageVariable(_, _, contract_no, var_no) => {
            ns.contracts[*contract_no].variables[*var_no].assigned = true;
        }

        AssemblyExpression::MemberAccess(_, member, _) => {
            assigned_variable(ns, member, symtable);
        }

        _ => (),
    }
}

pub(crate) fn used_variable(ns: &mut Namespace, exp: &AssemblyExpression, symtable: &mut Symtable) {
    match exp {
        AssemblyExpression::SolidityLocalVariable(_, _, _, var_no)
        | AssemblyExpression::AssemblyLocalVariable(_, _, var_no) => {
            let var = symtable.vars.get_mut(var_no).unwrap();
            (*var).read = true;
        }

        AssemblyExpression::StorageVariable(_, _, contract_no, var_no) => {
            ns.contracts[*contract_no].variables[*var_no].read = true;
        }

        AssemblyExpression::MemberAccess(_, member, _) => {
            used_variable(ns, member, symtable);
        }

        _ => (),
    }
}
