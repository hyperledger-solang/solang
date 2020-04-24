use super::{ContractVariable, Namespace, Symbol};
use output::Output;
use parser::ast;
use resolver::cfg::{ControlFlowGraph, Instr, Storage, Vartable};
use resolver::expression::{cast, expression, Expression};
use resolver::ContractVariableType;

pub fn contract_variables(
    def: &ast::ContractDefinition,
    contract_no: usize,
    ns: &mut Namespace,
    errors: &mut Vec<Output>,
) -> bool {
    let mut broken = false;
    let mut vartab = Vartable::new();
    let mut cfg = ControlFlowGraph::new();

    for parts in &def.parts {
        if let ast::ContractPart::ContractVariableDefinition(ref s) = parts {
            if !var_decl(s, contract_no, ns, &mut cfg, &mut vartab, errors) {
                broken = true;
            }
        }
    }

    cfg.add(&mut vartab, Instr::Return { value: Vec::new() });

    cfg.vars = vartab.drain();

    ns.contracts[contract_no].initializer = cfg;

    broken
}

fn var_decl(
    s: &ast::ContractVariableDefinition,
    contract_no: usize,
    ns: &mut Namespace,
    cfg: &mut ControlFlowGraph,
    vartab: &mut Vartable,
    errors: &mut Vec<Output>,
) -> bool {
    let ty = match ns.resolve_type(Some(contract_no), &s.ty, errors) {
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
                        *loc,
                        "duplicate constant attribute".to_string(),
                    ));
                }
                is_constant = true;
            }
            ast::VariableAttribute::Visibility(ast::Visibility::External(loc)) => {
                errors.push(Output::error(
                    *loc,
                    "variable cannot be declared external".to_string(),
                ));
                return false;
            }
            ast::VariableAttribute::Visibility(v) => {
                if let Some(e) = &visibility {
                    errors.push(Output::error_with_note(
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
        None => ast::Visibility::Private(ast::Loc(0, 0)),
    };

    let var = if !is_constant {
        let storage = ns.contracts[contract_no].top_of_contract_storage.clone();
        let slots = ty.storage_slots(ns);
        ns.contracts[contract_no].top_of_contract_storage += slots;
        ContractVariableType::Storage(storage)
    } else {
        ContractVariableType::Constant(ns.contracts[contract_no].constants.len())
    };

    let initializer = if let Some(initializer) = &s.initializer {
        let expr = if is_constant {
            expression(&initializer, cfg, Some(contract_no), ns, &mut None, errors)
        } else {
            expression(
                &initializer,
                cfg,
                Some(contract_no),
                ns,
                &mut Some(vartab),
                errors,
            )
        };

        let (res, resty) = match expr {
            Ok((res, ty)) => (res, ty),
            Err(()) => return false,
        };

        // implicityly conversion to correct ty
        let res = match cast(&s.loc, res, &resty, &ty, true, ns, errors) {
            Ok(res) => res,
            Err(_) => return false,
        };

        Some(res)
    } else {
        if is_constant {
            errors.push(Output::decl_error(
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
        ty: ty.clone(),
        var,
    };

    let pos = ns.contracts[contract_no].variables.len();

    ns.contracts[contract_no].variables.push(sdecl);

    if !ns.add_symbol(
        Some(contract_no),
        &s.name,
        Symbol::Variable(s.loc, pos),
        errors,
    ) {
        return false;
    }

    if let Some(res) = initializer {
        if is_constant {
            ns.contracts[contract_no].constants.push(res);
        } else {
            let var = vartab.find(&s.name, contract_no, ns, errors).unwrap();
            let loc = res.loc();

            cfg.add(
                vartab,
                Instr::Set {
                    res: var.pos,
                    expr: res,
                },
            );

            if let Storage::Contract(offset) = &var.storage {
                cfg.add(
                    vartab,
                    Instr::SetStorage {
                        ty,
                        local: var.pos,
                        storage: Expression::NumberLiteral(loc, 256, offset.clone()),
                    },
                );
            }
        }
    }

    true
}
