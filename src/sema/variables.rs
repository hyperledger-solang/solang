use super::ast::{ContractVariable, ContractVariableType, Namespace, Symbol};
use output::Output;
use parser::pt;
use sema::expression::{cast, expression};
use sema::symtable::Symtable;

pub fn contract_variables(
    def: &pt::ContractDefinition,
    contract_no: usize,
    ns: &mut Namespace,
) -> bool {
    let mut broken = false;
    let mut symtable = Symtable::new();

    for parts in &def.parts {
        if let pt::ContractPart::ContractVariableDefinition(ref s) = parts {
            if !var_decl(s, contract_no, ns, &mut symtable) {
                broken = true;
            }
        }
    }

    broken
}

fn var_decl(
    s: &pt::ContractVariableDefinition,
    contract_no: usize,
    ns: &mut Namespace,
    symtable: &mut Symtable,
) -> bool {
    let ty = match ns.resolve_type(Some(contract_no), false, &s.ty) {
        Ok(s) => s,
        Err(()) => {
            return false;
        }
    };

    let mut is_constant = false;
    let mut visibility: Option<pt::Visibility> = None;

    for attr in &s.attrs {
        match &attr {
            pt::VariableAttribute::Constant(loc) => {
                if is_constant {
                    ns.diagnostics.push(Output::warning(
                        *loc,
                        "duplicate constant attribute".to_string(),
                    ));
                }
                is_constant = true;
            }
            pt::VariableAttribute::Visibility(pt::Visibility::External(loc)) => {
                ns.diagnostics.push(Output::error(
                    *loc,
                    "variable cannot be declared external".to_string(),
                ));
                return false;
            }
            pt::VariableAttribute::Visibility(v) => {
                if let Some(e) = &visibility {
                    ns.diagnostics.push(Output::error_with_note(
                        v.loc(),
                        format!("variable visibility redeclared `{}'", v.to_string()),
                        e.loc(),
                        format!("location of previous declaration of `{}'", e.to_string()),
                    ));
                    return false;
                }

                visibility = Some(v.clone());
            }
        }
    }

    let visibility = match visibility {
        Some(v) => v,
        None => pt::Visibility::Private(pt::Loc(0, 0)),
    };

    let var = if !is_constant {
        let storage = ns.contracts[contract_no].top_of_contract_storage.clone();
        let slots = ty.storage_slots(ns);
        ns.contracts[contract_no].top_of_contract_storage += slots;
        ContractVariableType::Storage(storage)
    } else {
        ContractVariableType::Constant
    };

    let initializer = if let Some(initializer) = &s.initializer {
        let res = match expression(&initializer, Some(contract_no), ns, &symtable, is_constant) {
            Ok(res) => res,
            Err(()) => return false,
        };

        // implicitly conversion to correct ty
        let res = match cast(&s.loc, res, &ty, true, ns) {
            Ok(res) => res,
            Err(_) => return false,
        };

        Some(res)
    } else {
        if is_constant {
            ns.diagnostics.push(Output::decl_error(
                s.loc,
                "missing initializer for constant".to_string(),
            ));
            return false;
        }

        None
    };

    let sdecl = ContractVariable {
        name: s.name.name.to_string(),
        doc: s.doc.clone(),
        visibility,
        ty,
        var,
        initializer,
    };

    let pos = ns.contracts[contract_no].variables.len();

    ns.contracts[contract_no].variables.push(sdecl);

    ns.add_symbol(Some(contract_no), &s.name, Symbol::Variable(s.loc, pos))
}
