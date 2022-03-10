use super::ast::{
    BuiltinStruct, Diagnostic, Expression, Function, Namespace, Parameter, Statement, Symbol, Type,
    Variable,
};
use super::expression::{cast, expression, ExprContext, ResolveTo};
use super::symtable::Symtable;
use super::tags::resolve_tags;
use crate::parser::pt;
use crate::parser::pt::CodeLocation;
use crate::parser::pt::OptionalCodeLocation;

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

    let mut constant = false;
    let mut visibility: Option<pt::Visibility> = None;
    let mut has_immutable: Option<pt::Loc> = None;
    let mut has_override: Option<pt::Loc> = None;

    for attr in attrs {
        match &attr {
            pt::VariableAttribute::Constant(loc) => {
                if constant {
                    ns.diagnostics.push(Diagnostic::error(
                        *loc,
                        "duplicate constant attribute".to_string(),
                    ));
                }
                constant = true;
            }
            pt::VariableAttribute::Immutable(loc) => {
                if let Some(prev) = &has_immutable {
                    ns.diagnostics.push(Diagnostic::error_with_note(
                        *loc,
                        "duplicate ‘immutable’ attribute".to_string(),
                        *prev,
                        "previous ‘immutable’ attribute".to_string(),
                    ));
                }
                has_immutable = Some(*loc);
            }
            pt::VariableAttribute::Override(loc) => {
                if let Some(prev) = &has_override {
                    ns.diagnostics.push(Diagnostic::error_with_note(
                        *loc,
                        "duplicate ‘override’ attribute".to_string(),
                        *prev,
                        "previous ‘override’ attribute".to_string(),
                    ));
                }
                has_override = Some(*loc);
            }
            pt::VariableAttribute::Visibility(v) if contract_no.is_none() => {
                ns.diagnostics.push(Diagnostic::error(
                    v.loc().unwrap(),
                    format!("‘{}’: global variable cannot have visibility specifier", v),
                ));
                return None;
            }
            pt::VariableAttribute::Visibility(pt::Visibility::External(loc)) => {
                ns.diagnostics.push(Diagnostic::error(
                    loc.unwrap(),
                    "variable cannot be declared external".to_string(),
                ));
                return None;
            }
            pt::VariableAttribute::Visibility(v) => {
                if let Some(e) = &visibility {
                    ns.diagnostics.push(Diagnostic::error_with_note(
                        v.loc().unwrap(),
                        format!("variable visibility redeclared `{}'", v),
                        e.loc().unwrap(),
                        format!("location of previous declaration of `{}'", e),
                    ));
                    return None;
                }

                visibility = Some(v.clone());
            }
        }
    }

    if let Some(loc) = &has_immutable {
        if constant {
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                "variable cannot be declared both ‘immutable’ and ‘constant’".to_string(),
            ));
            constant = false;
        }
    }

    let visibility = match visibility {
        Some(v) => v,
        None => pt::Visibility::Internal(Some(s.ty.loc())),
    };

    if let pt::Visibility::Public(_) = &visibility {
        // override allowed
    } else if let Some(loc) = &has_override {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            "only public variable can be declared ‘override’".to_string(),
        ));
        has_override = None;
    }

    if contract_no.is_none() {
        if !constant {
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
    } else if let Some(ty) = ty.contains_builtins(ns, BuiltinStruct::AccountInfo) {
        let message = format!(
            "variable of cannot be builtin of type ‘{}’",
            ty.to_string(ns)
        );
        ns.diagnostics.push(Diagnostic::error(s.ty.loc(), message));
        return None;
    } else if let Some(ty) = ty.contains_builtins(ns, BuiltinStruct::AccountMeta) {
        let message = format!(
            "variable of cannot be builtin of type ‘{}’",
            ty.to_string(ns)
        );
        ns.diagnostics.push(Diagnostic::error(s.ty.loc(), message));
        return None;
    }

    let initializer = if let Some(initializer) = &s.initializer {
        let mut diagnostics = Vec::new();
        let context = ExprContext {
            file_no,
            unchecked: false,
            contract_no,
            function_no: None,
            constant,
            lvalue: false,
        };

        match expression(
            initializer,
            &context,
            ns,
            symtable,
            &mut diagnostics,
            ResolveTo::Type(&ty),
        ) {
            Ok(res) => {
                // implicitly conversion to correct ty
                match cast(&s.loc, res, &ty, true, ns, &mut diagnostics) {
                    Ok(res) => Some(res),
                    Err(_) => {
                        ns.diagnostics.extend(diagnostics);
                        None
                    }
                }
            }
            Err(()) => {
                ns.diagnostics.extend(diagnostics);
                None
            }
        }
    } else {
        if constant {
            ns.diagnostics.push(Diagnostic::decl_error(
                s.loc,
                "missing initializer for constant".to_string(),
            ));
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
        s.name.loc.file_no(),
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
        constant,
        immutable: has_immutable.is_some(),
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
            let mut expr = if constant {
                Expression::ConstantVariable(pt::Loc::Implicit, ty.clone(), Some(contract_no), pos)
            } else {
                Expression::StorageVariable(
                    pt::Loc::Implicit,
                    Type::StorageRef(false, Box::new(ty.clone())),
                    contract_no,
                    pos,
                )
            };

            // If the variable is an array or mapping, the accessor function takes mapping keys
            // or array indices as arguments, and returns the dereferenced value
            let mut params = Vec::new();
            let ty = collect_parameters(&ty, &mut params, &mut expr, ns);

            if ty.contains_mapping(ns) {
                // we can't return a mapping
                ns.diagnostics.push(Diagnostic::decl_error(
                    s.loc,
                    "mapping in a struct variable cannot be public".to_string(),
                ));
            }

            let mut func = Function::new(
                s.name.loc,
                s.name.name.to_owned(),
                Some(contract_no),
                Vec::new(),
                pt::FunctionTy::Function,
                // accessors for constant variables have view mutability
                Some(pt::Mutability::View(s.name.loc)),
                visibility,
                params,
                vec![Parameter {
                    name: None,
                    loc: s.name.loc,
                    ty: ty.clone(),
                    ty_loc: s.ty.loc(),
                    indexed: false,
                    readonly: false,
                }],
                ns,
            );

            // Create the implicit body - just return the value
            func.body = vec![Statement::Return(
                pt::Loc::Implicit,
                Some(if constant {
                    expr
                } else {
                    Expression::StorageLoad(pt::Loc::Implicit, ty.clone(), Box::new(expr))
                }),
            )];
            func.is_accessor = true;
            func.has_body = true;
            func.is_override = has_override.map(|loc| (loc, Vec::new()));

            // add the function to the namespace and then to our contract
            let func_no = ns.functions.len();

            ns.functions.push(func);

            ns.contracts[contract_no].functions.push(func_no);

            // we already have a symbol for
            let symbol = Symbol::Function(vec![(s.loc, func_no)]);

            ns.function_symbols.insert(
                (s.loc.file_no(), Some(contract_no), s.name.name.to_owned()),
                symbol,
            );
        }
    }

    // Return true if the value is constant
    Some(constant)
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
                pt::Loc::Implicit,
                ty.storage_array_elem(),
                Type::StorageRef(false, Box::new(ty.clone())),
                Box::new(map),
                Box::new(Expression::FunctionArg(
                    pt::Loc::Implicit,
                    key.as_ref().clone(),
                    params.len(),
                )),
            );

            params.push(Parameter {
                name: None,
                loc: pt::Loc::Implicit,
                ty: key.as_ref().clone(),
                ty_loc: pt::Loc::Implicit,
                indexed: false,
                readonly: false,
            });

            collect_parameters(value, params, expr, ns)
        }
        Type::Array(elem_ty, dims) => {
            let mut ty = Type::StorageRef(false, Box::new(ty.clone()));
            for _ in 0..dims.len() {
                let map = (*expr).clone();

                *expr = Expression::Subscript(
                    pt::Loc::Implicit,
                    ty.storage_array_elem(),
                    ty.clone(),
                    Box::new(map),
                    Box::new(Expression::FunctionArg(
                        pt::Loc::Implicit,
                        Type::Uint(256),
                        params.len(),
                    )),
                );

                ty = ty.storage_array_elem();

                params.push(Parameter {
                    name: None,
                    loc: pt::Loc::Implicit,
                    ty: Type::Uint(256),
                    ty_loc: pt::Loc::Implicit,
                    indexed: false,
                    readonly: false,
                });
            }

            collect_parameters(elem_ty, params, expr, ns)
        }
        _ => ty,
    }
}
