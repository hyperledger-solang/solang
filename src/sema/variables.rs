use super::ast::{Diagnostic, Namespace, Symbol, Variable};
use super::expression::{cast, expression};
use super::symtable::Symtable;
use super::tags::resolve_tags;
use crate::parser::pt;

pub fn contract_variables(
    def: &pt::ContractDefinition,
    file_no: usize,
    contract_no: usize,
    ns: &mut Namespace,
) -> bool {
    let mut broken = false;
    let mut symtable = Symtable::new();
    let may_have_state = !matches!(def.ty,
        pt::ContractTy::Interface(_) | pt::ContractTy::Library(_));

    for parts in &def.parts {
        if let pt::ContractPart::VariableDefinition(ref s) = parts {
            if !may_have_state {
                ns.diagnostics.push(Diagnostic::error(
                    s.loc,
                    format!(
                        "{} ‘{}’ is not allowed to have state variable ‘{}’",
                        def.ty, def.name.name, s.name.name
                    ),
                ));
            } else if !var_decl(Some(def), s, file_no, Some(contract_no), ns, &mut symtable) {
                broken = true;
            }
        }
    }

    broken
}

pub fn var_decl(
    contract: Option<&pt::ContractDefinition>,
    s: &pt::VariableDefinition,
    file_no: usize,
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &mut Symtable,
) -> bool {
    let mut attrs = s.attrs.clone();
    let mut ty = s.ty.clone();

    // For function types, the parser adds the attributes incl visibility to the type,
    // not the pt::VariableDefinition attrs. We need to chomp off the visibility
    // from the attributes before resolving the type
    if let pt::Expression::Type(
        _,
        pt::Type::Function {
            attributes,
            trailing_attributes,
            returns,
            ..
        },
    ) = &mut ty
    {
        if let Some(pt::FunctionAttribute::Visibility(v)) = trailing_attributes.last() {
            attrs.push(pt::VariableAttribute::Visibility(v.clone()));
            trailing_attributes.pop();
        } else if returns.is_empty() {
            if let Some(pt::FunctionAttribute::Visibility(v)) = attributes.last() {
                attrs.push(pt::VariableAttribute::Visibility(v.clone()));
                attributes.pop();
            }
        }
    }

    let ty = match ns.resolve_type(file_no, contract_no, false, &ty) {
        Ok(s) => s,
        Err(()) => {
            return false;
        }
    };

    let mut is_constant = false;
    let mut visibility: Option<pt::Visibility> = None;

    for attr in attrs {
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
            pt::VariableAttribute::Visibility(v) if contract_no.is_none() => {
                ns.diagnostics.push(Diagnostic::error(
                    v.loc(),
                    format!("‘{}’: global variable cannot have visibility specifier", v),
                ));
                return false;
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
        None => pt::Visibility::Internal(s.ty.loc()),
    };

    if contract_no.is_none() {
        if !is_constant {
            ns.diagnostics.push(Diagnostic::error(
                s.ty.loc(),
                "global variable must be constant".to_string(),
            ));
            return false;
        }
        if ty.contains_internal_function(ns) {
            ns.diagnostics.push(Diagnostic::error(
                s.ty.loc(),
                "global variable cannot be of type internal function".to_string(),
            ));
            return false;
        }
    } else if ty.contains_internal_function(ns)
        && matches!(visibility, pt::Visibility::Public(_) | pt::Visibility::External(_))
    {
        ns.diagnostics.push(Diagnostic::error(
            s.ty.loc(),
            format!(
                "variable of type internal function cannot be ‘{}’",
                visibility
            ),
        ));
        return false;
    }

    let initializer = if let Some(initializer) = &s.initializer {
        let res = match expression(
            &initializer,
            file_no,
            contract_no,
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

    let bases: Vec<&str> = if let Some(contract) = contract {
        contract
            .base
            .iter()
            .map(|base| -> &str { &base.name.name })
            .collect()
    } else {
        Vec::new()
    };

    let tags = resolve_tags(
        s.name.loc.0,
        if contract_no.is_none() {
            "global variable"
        } else {
            "state variable"
        },
        &s.doc,
        None,
        None,
        Some(&bases),
        ns,
    );

    let sdecl = Variable {
        name: s.name.name.to_string(),
        loc: s.loc,
        tags,
        visibility,
        ty,
        constant: is_constant,
        initializer,
    };

    let pos = if let Some(contract_no) = contract_no {
        let pos = ns.contracts[contract_no].variables.len();

        ns.contracts[contract_no].variables.push(sdecl);

        pos
    } else {
        let pos = ns.constants.len();

        ns.constants.push(sdecl);

        pos
    };

    ns.add_symbol(
        file_no,
        contract_no,
        &s.name,
        Symbol::Variable(s.loc, contract_no, pos),
    )
}
