
use parser::ast;
use output::Output;
use super::{Contract, ContractVariable, Symbol};

pub fn contract_variables(
    def: &ast::ContractDefinition,
    ns: &mut Contract,
    errors: &mut Vec<Output>
) -> bool {
    let mut broken = false;

    for parts in &def.parts {
        if let ast::ContractPart::ContractVariableDefinition(ref s) = parts {
            if !var_decl(s, ns, errors) {
                broken = true;
            }
        }
    }

    broken
}

fn var_decl(
    s: &ast::ContractVariableDefinition,
    ns: &mut Contract,
    errors: &mut Vec<Output>,
) -> bool {
    let ty = match ns.resolve_type(&s.ty, errors) {
        Ok(s) => s,
        Err(()) => {
            return false;
        }
    };

    let mut is_constant = false;
    let mut visibility: Option<ast::Visibility> = None;

    for attr in &s.attrs {
        match &attr {
            ast::VariableAttribute::Constant(loc) => {
                if is_constant {
                    errors.push(Output::warning(
                        loc.clone(),
                        format!("duplicate constant attribute"),
                    ));
                }
                is_constant = true;
            }
            ast::VariableAttribute::Visibility(ast::Visibility::External(loc)) => {
                errors.push(Output::error(
                    loc.clone(),
                    format!("variable cannot be declared external"),
                ));
                return false;
            }
            ast::VariableAttribute::Visibility(v) => {
                if let Some(e) = &visibility {
                    errors.push(Output::error_with_note(
                        v.loc().clone(),
                        format!("variable visibility redeclared `{}'", v.to_string()),
                        e.loc().clone(),
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
        None => ast::Visibility::Private(ast::Loc(0, 0)),
    };

    if is_constant && s.initializer == None {
        errors.push(Output::decl_error(
            s.loc.clone(),
            format!("missing initializer for constant"),
        ));
        return false;
    }

    let storage = if !is_constant {
        let storage = ns.top_of_contract_storage;
        ns.top_of_contract_storage += 1;
        Some(storage)
    } else {
        None
    };

    let sdecl = ContractVariable {
        name: s.name.name.to_string(),
        storage,
        visibility,
        ty,
    };

    // FIXME: resolve init expression and check for constant (if constant)
    // init expression can call functions and access other state variables

    let pos = ns.variables.len();

    ns.variables.push(sdecl);

    ns.add_symbol(&s.name, Symbol::Variable(s.loc, pos), errors)
}
