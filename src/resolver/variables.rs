
use parser::ast;
use output::Output;
use super::{Contract, ContractVariable, Symbol};
use resolver::cfg::{ControlFlowGraph, Vartable, Instr, set_contract_storage, expression, cast};
use resolver::ContractVariableType;

pub fn contract_variables(
    def: &ast::ContractDefinition,
    ns: &mut Contract,
    errors: &mut Vec<Output>
) -> bool {
    let mut broken = false;
    let mut vartab = Vartable::new();
    let mut cfg = ControlFlowGraph::new();

    for parts in &def.parts {
        if let ast::ContractPart::ContractVariableDefinition(ref s) = parts {
            if !var_decl(s, ns, &mut cfg, &mut vartab, errors) {
                broken = true;
            }
        }
    }

    cfg.add(&mut vartab, Instr::Return{ value: Vec::new() });

    cfg.vars = vartab.drain();

    ns.initializer = cfg;

    broken
}

fn var_decl(
    s: &ast::ContractVariableDefinition,
    ns: &mut Contract,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
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

    let var = if !is_constant {
        let storage = ns.top_of_contract_storage;
        ns.top_of_contract_storage += 1;
        ContractVariableType::Storage(storage)
    } else {
        ContractVariableType::Constant(ns.constants.len())
    };

    let initializer = if let Some(initializer) = &s.initializer {
        let expr = if is_constant {
            expression(&initializer, cfg, &ns, &mut None, errors)
        } else {
            expression(&initializer, cfg, &ns, &mut Some(vartab), errors)
        };

        let (res, resty) = match expr {
            Ok((res, ty)) => (res, ty),
            Err(()) => return false
        };

        // implicityly conversion to correct ty
        let res = match cast(&s.loc, res, &resty, &ty, true, &ns, errors) {
            Ok(res) => res,
            Err(_) => return false
        };

        Some(res)
    } else {
        if is_constant {
            errors.push(Output::decl_error(
                s.loc.clone(),
                format!("missing initializer for constant"),
            ));
            return false;
        }

        None
    };

    let sdecl = ContractVariable {
        name: s.name.name.to_string(),
        doc: s.doc.to_string(),
        visibility,
        ty,
        var
    };

    let pos = ns.variables.len();

    ns.variables.push(sdecl);

    if !ns.add_symbol(&s.name, Symbol::Variable(s.loc, pos), errors) {
        return false
    }

    if let Some(res) = initializer {
        if is_constant {
            ns.constants.push(res);
        } else {
            let var = vartab.find(&s.name, ns, errors).unwrap();

            cfg.add(vartab, Instr::Set{ res: var.pos, expr: res });

            set_contract_storage(&s.name, &var, cfg, vartab, errors).unwrap();
        }
    }

    true
}
