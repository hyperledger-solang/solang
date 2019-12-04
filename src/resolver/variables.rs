
use parser::ast;
use output::Output;
use super::{Contract, ContractVariable, Symbol};
use resolver::cfg::{ControlFlowGraph, Vartable, Instr, expression, cast};

pub fn contract_variables(
    def: &ast::ContractDefinition,
    ns: &mut Contract,
    errors: &mut Vec<Output>
) -> bool {
    let mut broken = false;
    let mut cfg = ControlFlowGraph::new();

    for parts in &def.parts {
        if let ast::ContractPart::ContractVariableDefinition(ref s) = parts {
            if !var_decl(s, ns, &mut cfg, errors) {
                broken = true;
            }
        }
    }

    ns.initializer = cfg;

    broken
}

fn var_decl(
    s: &ast::ContractVariableDefinition,
    ns: &mut Contract,
    cfg: &mut ControlFlowGraph,
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

    let storage = if !is_constant {
        let storage = ns.top_of_contract_storage;
        ns.top_of_contract_storage += 1;
        Some(storage)
    } else {
        None
    };

    if let Some(initializer) = &s.initializer {
        if is_constant {
            // TODO check for non-constant stuff
        }  else {
            let mut vartab = Vartable::new(ns);

            let (res, resty) = match expression(&initializer, cfg, &ns, &mut vartab, errors) {
                Ok((res, ty)) => (res, ty),
                Err(()) => return false
            };

            // implicityly convversion to correct ty
            let res = match cast(&s.loc, res, &resty, &ty, false, &ns, errors) {
                Ok(res) => res,
                Err(_) => return false
            };

            let temp_var = vartab.temp(&s.name, &ty);

            cfg.add(&mut vartab, Instr::Set{ res: temp_var, expr: res });
            cfg.add(&mut vartab, Instr::SetStorage{ local: temp_var, storage: storage.unwrap() });
        }
    } else {
        if is_constant {
            errors.push(Output::decl_error(
                s.loc.clone(),
                format!("missing initializer for constant"),
            ));
            return false;
        }
    }

    let sdecl = ContractVariable {
        name: s.name.name.to_string(),
        storage,
        visibility,
        ty,
    };

    let pos = ns.variables.len();

    ns.variables.push(sdecl);

    ns.add_symbol(&s.name, Symbol::Variable(s.loc, pos), errors)
}
