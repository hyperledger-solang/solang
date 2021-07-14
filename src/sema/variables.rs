use super::ast::{
    Diagnostic, Expression, Function, Namespace, Parameter, Statement, Symbol, Type, Variable,
};
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

    for parts in &def.parts {
        if let pt::ContractPart::VariableDefinition(ref s) = parts {
            // don't even attempt to parse contract variables for interfaces, they are never allowed
            if matches!(def.ty, pt::ContractTy::Interface(_)) {
                ns.diagnostics.push(Diagnostic::error(
                    s.loc,
                    format!(
                        "{} ‘{}’ is not allowed to have contract variable ‘{}’",
                        def.ty, def.name.name, s.name.name
                    ),
                ));
                broken = true;
                continue;
            }

            match var_decl(Some(def), s, file_no, Some(contract_no), ns, &mut symtable) {
                None => {
                    broken = true;
                }
                Some(false) if matches!(def.ty, pt::ContractTy::Library(_)) => {
                    ns.diagnostics.push(Diagnostic::error(
                        s.loc,
                        format!(
                            "{} ‘{}’ is not allowed to have state variable ‘{}’",
                            def.ty, def.name.name, s.name.name
                        ),
                    ));
                }
                _ => (),
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
) -> Option<bool> {
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

    let mut diagnostics = Vec::new();

    let ty = match ns.resolve_type(file_no, contract_no, false, &ty, &mut diagnostics) {
        Ok(s) => s,
        Err(()) => {
            ns.diagnostics.extend(diagnostics);
            return None;
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
                return None;
            }
            pt::VariableAttribute::Visibility(pt::Visibility::External(loc)) => {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "variable cannot be declared external".to_string(),
                ));
                return None;
            }
            pt::VariableAttribute::Visibility(v) => {
                if let Some(e) = &visibility {
                    ns.diagnostics.push(Diagnostic::error_with_note(
                        v.loc(),
                        format!("variable visibility redeclared `{}'", v.to_string()),
                        e.loc(),
                        format!("location of previous declaration of `{}'", e.to_string()),
                    ));
                    return None;
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
            return None;
        }
        if ty.contains_internal_function(ns) {
            ns.diagnostics.push(Diagnostic::error(
                s.ty.loc(),
                "global variable cannot be of type internal function".to_string(),
            ));
            return None;
        }
    } else if ty.contains_internal_function(ns)
        && matches!(
            visibility,
            pt::Visibility::Public(_) | pt::Visibility::External(_)
        )
    {
        ns.diagnostics.push(Diagnostic::error(
            s.ty.loc(),
            format!(
                "variable of type internal function cannot be ‘{}’",
                visibility
            ),
        ));
        return None;
    }

    let initializer = if let Some(initializer) = &s.initializer {
        let mut diagnostics = Vec::new();

        let res = match expression(
            &initializer,
            file_no,
            contract_no,
            ns,
            symtable,
            is_constant,
            &mut diagnostics,
            Some(&ty),
        ) {
            Ok(res) => res,
            Err(()) => {
                ns.diagnostics.extend(diagnostics);
                return None;
            }
        };

        // implicitly conversion to correct ty
        let res = match cast(&s.loc, res, &ty, true, ns, &mut diagnostics) {
            Ok(res) => res,
            Err(_) => {
                ns.diagnostics.extend(diagnostics);
                return None;
            }
        };

        Some(res)
    } else {
        if is_constant {
            ns.diagnostics.push(Diagnostic::decl_error(
                s.loc,
                "missing initializer for constant".to_string(),
            ));
            return None;
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
        visibility: visibility.clone(),
        ty: ty.clone(),
        constant: is_constant,
        assigned: initializer.is_some(),
        initializer,
        read: matches!(visibility, pt::Visibility::Public(_)),
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

    let success = ns.add_symbol(
        file_no,
        contract_no,
        &s.name,
        Symbol::Variable(s.loc, contract_no, pos),
    );

    // for public variables in contracts, create an accessor function
    if success && matches!(visibility, pt::Visibility::Public(_)) {
        if let Some(contract_no) = contract_no {
            // The accessor function returns the value of the storage variable, constant or not.
            let mut expr = if is_constant {
                Expression::ConstantVariable(pt::Loc(0, 0, 0), ty.clone(), Some(contract_no), pos)
            } else {
                Expression::StorageVariable(
                    pt::Loc(0, 0, 0),
                    Type::StorageRef(Box::new(ty.clone())),
                    contract_no,
                    pos,
                )
            };

            // If the variable is an array or mapping, the accessor function takes mapping keys
            // or array indices as arguments, and returns the dereferenced value
            let mut params = Vec::new();
            let ty = collect_parameters(&ty, &mut params, &mut expr, ns);

            let mut func = Function::new(
                s.name.loc,
                s.name.name.to_owned(),
                Some(contract_no),
                Vec::new(),
                pt::FunctionTy::Function,
                // accessors for constant variables have view mutability
                Some(pt::StateMutability::View(s.name.loc)),
                visibility,
                params,
                vec![Parameter {
                    name: String::new(),
                    name_loc: Some(s.name.loc),
                    loc: s.name.loc,
                    ty: ty.clone(),
                    ty_loc: s.ty.loc(),
                    indexed: false,
                }],
                ns,
            );

            // Create the implicit body - just return the value
            func.body = vec![Statement::Return(
                pt::Loc(0, 0, 0),
                vec![if is_constant {
                    expr
                } else {
                    Expression::StorageLoad(pt::Loc(0, 0, 0), ty.clone(), Box::new(expr))
                }],
            )];
            func.is_accessor = true;
            func.has_body = true;

            // add the function to the namespace and then to our contract
            let func_no = ns.functions.len();

            ns.functions.push(func);

            ns.contracts[contract_no].functions.push(func_no);

            // we already have a symbol for
            let symbol = Symbol::Function(vec![(s.loc, func_no)]);

            ns.function_symbols
                .insert((s.loc.0, Some(contract_no), s.name.name.to_owned()), symbol);
        }
    }

    // Return true if the value is constant
    Some(is_constant)
}

/// For accessor functions, create the parameter list and the return expression
fn collect_parameters<'a>(
    ty: &'a Type,
    params: &mut Vec<Parameter>,
    expr: &mut Expression,
    ns: &Namespace,
) -> &'a Type {
    match ty {
        Type::Mapping(key, value) => {
            let map = (*expr).clone();

            *expr = Expression::Subscript(
                pt::Loc(0, 0, 0),
                Type::StorageRef(Box::new(ty.clone())),
                Box::new(map),
                Box::new(Expression::FunctionArg(
                    pt::Loc(0, 0, 0),
                    key.as_ref().clone(),
                    params.len(),
                )),
            );

            params.push(Parameter {
                name: String::new(),
                name_loc: None,
                loc: pt::Loc(0, 0, 0),
                ty: key.as_ref().clone(),
                ty_loc: pt::Loc(0, 0, 0),
                indexed: false,
            });

            collect_parameters(value, params, expr, ns)
        }
        Type::Array(elem_ty, dims) => {
            let mut ty = Type::StorageRef(Box::new(ty.clone()));
            for _ in 0..dims.len() {
                let map = (*expr).clone();

                *expr = Expression::Subscript(
                    pt::Loc(0, 0, 0),
                    ty.clone(),
                    Box::new(map),
                    Box::new(Expression::FunctionArg(
                        pt::Loc(0, 0, 0),
                        Type::Uint(256),
                        params.len(),
                    )),
                );

                ty = ty.storage_array_elem();

                params.push(Parameter {
                    name: String::new(),
                    name_loc: None,
                    loc: pt::Loc(0, 0, 0),
                    ty: Type::Uint(256),
                    ty_loc: pt::Loc(0, 0, 0),
                    indexed: false,
                });
            }

            collect_parameters(elem_ty, params, expr, ns)
        }
        _ => ty,
    }
}
