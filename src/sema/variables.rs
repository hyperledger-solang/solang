use super::ast::{ContractVariable, ContractVariableType, Diagnostic, Namespace, Symbol};
use super::expression::{cast, expression};
use super::symtable::Symtable;
use parser::pt;

pub fn contract_variables(
    def: &pt::ContractDefinition,
    file_no: usize,
    contract_no: usize,
    ns: &mut Namespace,
) -> bool {
    let mut broken = false;
    let mut symtable = Symtable::new();
    let may_have_state = match def.ty {
        pt::ContractTy::Interface(_) | pt::ContractTy::Library(_) => false,
        _ => true,
    };

    for parts in &def.parts {
        if let pt::ContractPart::ContractVariableDefinition(ref s) = parts {
            if !may_have_state {
                ns.diagnostics.push(Diagnostic::error(
                    s.loc,
                    format!(
                        "{} ‘{}’ is not allowed to have state variable ‘{}’",
                        def.ty, def.name.name, s.name.name
                    ),
                ));
            } else if !var_decl(s, file_no, contract_no, ns, &mut symtable) {
                broken = true;
            }
        }
    }

    broken
}

fn var_decl(
    s: &pt::ContractVariableDefinition,
    file_no: usize,
    contract_no: usize,
    ns: &mut Namespace,
    symtable: &mut Symtable,
) -> bool {
    let ty = match ns.resolve_type(file_no, Some(contract_no), false, &s.ty) {
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
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        "duplicate constant attribute".to_string(),
                    ));
                }
                is_constant = true;
            }
            pt::VariableAttribute::Visibility(pt::Visibility::External(loc)) => {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "variable cannot be declared external".to_string(),
                ));
                return false;
            }
            pt::VariableAttribute::Visibility(v) => {
                if let Some(e) = &visibility {
                    ns.diagnostics.push(Diagnostic::error_with_note(
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
        None => pt::Visibility::Private(pt::Loc(0, 0, 0)),
    };

    let var = if !is_constant {
        ContractVariableType::Storage
    } else {
        ContractVariableType::Constant
    };

    let initializer = if let Some(initializer) = &s.initializer {
        let res = match expression(
            &initializer,
            file_no,
            Some(contract_no),
            ns,
            &symtable,
            is_constant,
        ) {
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
            ns.diagnostics.push(Diagnostic::decl_error(
                s.loc,
                "missing initializer for constant".to_string(),
            ));
            return false;
        }

        None
    };

    let sdecl = ContractVariable {
        name: s.name.name.to_string(),
        loc: s.loc,
        doc: s.doc.clone(),
        visibility,
        ty,
        var,
        initializer,
    };

    let pos = ns.contracts[contract_no].variables.len();

    ns.contracts[contract_no].variables.push(sdecl);

    ns.add_symbol(
        file_no,
        Some(contract_no),
        &s.name,
        Symbol::Variable(s.loc, contract_no, pos),
    )
}
