use super::{
    ast::{
        BuiltinStruct, Diagnostic, Expression, Function, Namespace, Parameter, Statement, Symbol,
        Type, Variable,
    },
    contracts::is_base,
    expression::{expression, ExprContext, ResolveTo},
    symtable::Symtable,
    symtable::{VariableInitializer, VariableUsage},
    tags::{parse_doccomments, resolve_tags, DocComment},
};
use solang_parser::pt::{self, CodeLocation, OptionalCodeLocation};

pub struct DelayedResolveInitializer<'a> {
    var_no: usize,
    contract_no: usize,
    initializer: &'a pt::Expression,
}

pub fn contract_variables<'a>(
    def: &'a pt::ContractDefinition,
    comments: &[pt::Comment],
    file_no: usize,
    contract_no: usize,
    ns: &mut Namespace,
) -> Vec<DelayedResolveInitializer<'a>> {
    let mut symtable = Symtable::new();
    let mut delayed = Vec::new();
    let mut doc_comment_start = def.loc.start();

    for part in &def.parts {
        match part {
            pt::ContractPart::VariableDefinition(ref s) => {
                let tags = parse_doccomments(comments, doc_comment_start, s.loc.start());

                if let Some(delay) = variable_decl(
                    Some(def),
                    s,
                    file_no,
                    &tags,
                    Some(contract_no),
                    ns,
                    &mut symtable,
                ) {
                    delayed.push(delay);
                }
            }
            pt::ContractPart::FunctionDefinition(f) => {
                if let Some(pt::Statement::Block { loc, .. }) = &f.body {
                    doc_comment_start = loc.end();
                    continue;
                }
            }
            _ => (),
        }

        doc_comment_start = part.loc().end();
    }

    delayed
}

pub fn variable_decl<'a>(
    contract: Option<&pt::ContractDefinition>,
    def: &'a pt::VariableDefinition,
    file_no: usize,
    tags: &[DocComment],
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &mut Symtable,
) -> Option<DelayedResolveInitializer<'a>> {
    let mut attrs = def.attrs.clone();
    let mut ty = def.ty.clone();
    let mut ret = None;

    // For function types, the parser adds the attributes incl visibility to the type,
    // not the pt::VariableDefinition attrs. We need to chomp off the visibility
    // from the attributes before resolving the type
    if let pt::Expression::Type(
        _,
        pt::Type::Function {
            attributes,
            returns,
            ..
        },
    ) = &mut ty
    {
        let mut filter_var_attrs = |attributes: &mut Vec<pt::FunctionAttribute>| {
            if attributes.is_empty() {
                return;
            }

            let mut seen_visibility = false;

            // here we must iterate in reverse order: we can only remove the *last* visibility attribute
            // This is due to the insane syntax
            // contract c {
            //    function() external internal x;
            // }
            // The first external means the function type is external, the second internal means the visibility
            // of the x is internal.
            for attr_no in (0..attributes.len()).rev() {
                if let pt::FunctionAttribute::Immutable(loc) = &attributes[attr_no] {
                    attrs.push(pt::VariableAttribute::Immutable(*loc));
                    attributes.remove(attr_no);
                } else if !seen_visibility {
                    if let pt::FunctionAttribute::Visibility(v) = &attributes[attr_no] {
                        attrs.push(pt::VariableAttribute::Visibility(v.clone()));
                        attributes.remove(attr_no);
                        seen_visibility = true;
                    }
                }
            }
        };

        if let Some((_, trailing_attributes)) = returns {
            filter_var_attrs(trailing_attributes);
        } else {
            filter_var_attrs(attributes);
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
    let mut is_override: Option<(pt::Loc, Vec<usize>)> = None;

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
                        "duplicate 'immutable' attribute".to_string(),
                        *prev,
                        "previous 'immutable' attribute".to_string(),
                    ));
                }
                has_immutable = Some(*loc);
            }
            pt::VariableAttribute::Override(loc, bases) => {
                if let Some((prev, _)) = &is_override {
                    ns.diagnostics.push(Diagnostic::error_with_note(
                        *loc,
                        "duplicate 'override' attribute".to_string(),
                        *prev,
                        "previous 'override' attribute".to_string(),
                    ));
                }

                let mut list = Vec::new();
                let mut diagnostics = Vec::new();

                if let Some(contract_no) = contract_no {
                    for name in bases {
                        if let Ok(no) =
                            ns.resolve_contract_with_namespace(file_no, name, &mut diagnostics)
                        {
                            if list.contains(&no) {
                                diagnostics.push(Diagnostic::error(
                                    name.loc,
                                    format!("duplicate override '{}'", name),
                                ));
                            } else if !is_base(no, contract_no, ns) {
                                diagnostics.push(Diagnostic::error(
                                    name.loc,
                                    format!(
                                        "override '{}' is not a base contract of '{}'",
                                        name, ns.contracts[contract_no].name
                                    ),
                                ));
                            } else {
                                list.push(no);
                            }
                        }
                    }

                    is_override = Some((*loc, list));
                } else {
                    diagnostics.push(Diagnostic::error(
                        *loc,
                        "global variable has no bases contracts to override".to_string(),
                    ));
                }

                ns.diagnostics.extend(diagnostics);
            }
            pt::VariableAttribute::Visibility(v) if contract_no.is_none() => {
                ns.diagnostics.push(Diagnostic::error(
                    v.loc().unwrap(),
                    format!("'{}': global variable cannot have visibility specifier", v),
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
                        format!("variable visibility redeclared '{}'", v),
                        e.loc().unwrap(),
                        format!("location of previous declaration of '{}'", e),
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
                "variable cannot be declared both 'immutable' and 'constant'".to_string(),
            ));
            constant = false;
        }
    }

    let visibility = match visibility {
        Some(v) => v,
        None => pt::Visibility::Internal(Some(def.ty.loc())),
    };

    if let pt::Visibility::Public(_) = &visibility {
        // override allowed
    } else if let Some((loc, _)) = &is_override {
        ns.diagnostics.push(Diagnostic::error(
            *loc,
            "only public variable can be declared 'override'".to_string(),
        ));
        is_override = None;
    }

    if let Some(contract) = contract {
        if matches!(contract.ty, pt::ContractTy::Interface(_))
            || (matches!(contract.ty, pt::ContractTy::Library(_)) && !constant)
        {
            ns.diagnostics.push(Diagnostic::error(
                def.loc,
                format!(
                    "{} '{}' is not allowed to have contract variable '{}'",
                    contract.ty, contract.name.name, def.name.name
                ),
            ));
            return None;
        }
    } else {
        if !constant {
            ns.diagnostics.push(Diagnostic::error(
                def.ty.loc(),
                "global variable must be constant".to_string(),
            ));
            return None;
        }
        if ty.contains_internal_function(ns) {
            ns.diagnostics.push(Diagnostic::error(
                def.ty.loc(),
                "global variable cannot be of type internal function".to_string(),
            ));
            return None;
        }
    }

    if ty.contains_internal_function(ns)
        && matches!(
            visibility,
            pt::Visibility::Public(_) | pt::Visibility::External(_)
        )
    {
        ns.diagnostics.push(Diagnostic::error(
            def.ty.loc(),
            format!(
                "variable of type internal function cannot be '{}'",
                visibility
            ),
        ));
        return None;
    } else if let Some(ty) = ty.contains_builtins(ns, BuiltinStruct::AccountInfo) {
        let message = format!("variable cannot be of builtin type '{}'", ty.to_string(ns));
        ns.diagnostics
            .push(Diagnostic::error(def.ty.loc(), message));
        return None;
    } else if let Some(ty) = ty.contains_builtins(ns, BuiltinStruct::AccountMeta) {
        let message = format!("variable cannot be of builtin type '{}'", ty.to_string(ns));
        ns.diagnostics
            .push(Diagnostic::error(def.ty.loc(), message));
        return None;
    }

    let initializer = if constant {
        if let Some(initializer) = &def.initializer {
            let mut diagnostics = Vec::new();
            let context = ExprContext {
                file_no,
                unchecked: false,
                contract_no,
                function_no: None,
                constant,
                lvalue: false,
                yul_function: false,
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
                    match res.cast(&def.loc, &ty, true, ns, &mut diagnostics) {
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
            ns.diagnostics.push(Diagnostic::decl_error(
                def.loc,
                "missing initializer for constant".to_string(),
            ));

            None
        }
    } else {
        None
    };

    let bases = if let Some(contract) = contract {
        contract
            .base
            .iter()
            .map(|base| format!("{}", base.name))
            .collect()
    } else {
        Vec::new()
    };

    let tags = resolve_tags(
        def.name.loc.file_no(),
        if contract_no.is_none() {
            "global variable"
        } else {
            "state variable"
        },
        tags,
        None,
        None,
        Some(&bases),
        ns,
    );

    let sdecl = Variable {
        name: def.name.name.to_string(),
        loc: def.loc,
        tags,
        visibility: visibility.clone(),
        ty: ty.clone(),
        constant,
        immutable: has_immutable.is_some(),
        assigned: def.initializer.is_some(),
        initializer,
        read: matches!(visibility, pt::Visibility::Public(_)),
    };

    let var_no = if let Some(contract_no) = contract_no {
        let var_no = ns.contracts[contract_no].variables.len();

        ns.contracts[contract_no].variables.push(sdecl);

        if !constant {
            if let Some(initializer) = &def.initializer {
                ret = Some(DelayedResolveInitializer {
                    var_no,
                    contract_no,
                    initializer,
                });
            }
        }

        var_no
    } else {
        let var_no = ns.constants.len();

        ns.constants.push(sdecl);

        var_no
    };

    let success = ns.add_symbol(
        file_no,
        contract_no,
        &def.name,
        Symbol::Variable(def.loc, contract_no, var_no),
    );

    // for public variables in contracts, create an accessor function
    if success && matches!(visibility, pt::Visibility::Public(_)) {
        if let Some(contract_no) = contract_no {
            // The accessor function returns the value of the storage variable, constant or not.
            let mut expr = if constant {
                Expression::ConstantVariable(
                    pt::Loc::Implicit,
                    ty.clone(),
                    Some(contract_no),
                    var_no,
                )
            } else {
                Expression::StorageVariable(
                    pt::Loc::Implicit,
                    Type::StorageRef(false, Box::new(ty.clone())),
                    contract_no,
                    var_no,
                )
            };

            // If the variable is an array or mapping, the accessor function takes mapping keys
            // or array indices as arguments, and returns the dereferenced value
            let mut symtable = Symtable::new();
            let mut params = Vec::new();
            let ty = collect_parameters(&ty, &mut symtable, &mut params, &mut expr, ns);

            if ty.contains_mapping(ns) {
                // we can't return a mapping
                ns.diagnostics.push(Diagnostic::decl_error(
                    def.loc,
                    "mapping in a struct variable cannot be public".to_string(),
                ));
            }

            let mut func = Function::new(
                def.name.loc,
                def.name.name.to_owned(),
                Some(contract_no),
                Vec::new(),
                pt::FunctionTy::Function,
                // accessors for constant variables have view mutability
                Some(pt::Mutability::View(def.name.loc)),
                visibility,
                params,
                vec![Parameter {
                    id: None,
                    loc: def.name.loc,
                    ty: ty.clone(),
                    ty_loc: Some(def.ty.loc()),
                    indexed: false,
                    readonly: false,
                    recursive: false,
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
            func.is_override = is_override;
            func.symtable = symtable;

            // add the function to the namespace and then to our contract
            let func_no = ns.functions.len();

            ns.functions.push(func);

            ns.contracts[contract_no].functions.push(func_no);

            // we already have a symbol for
            let symbol = Symbol::Function(vec![(def.loc, func_no)]);

            ns.function_symbols.insert(
                (
                    def.loc.file_no(),
                    Some(contract_no),
                    def.name.name.to_owned(),
                ),
                symbol,
            );
        }
    }

    ret
}

/// For accessor functions, create the parameter list and the return expression
fn collect_parameters<'a>(
    ty: &'a Type,
    symtable: &mut Symtable,
    params: &mut Vec<Parameter>,
    expr: &mut Expression,
    ns: &mut Namespace,
) -> &'a Type {
    match ty {
        Type::Mapping(key, value) => {
            let map = (*expr).clone();

            let id = pt::Identifier {
                loc: pt::Loc::Implicit,
                name: "".to_owned(),
            };
            let arg_ty = key.as_ref().clone();

            let arg_no = symtable
                .add(
                    &id,
                    arg_ty.clone(),
                    ns,
                    VariableInitializer::Solidity(None),
                    VariableUsage::Parameter,
                    None,
                )
                .unwrap();

            symtable.arguments.push(Some(arg_no));

            *expr = Expression::Subscript(
                pt::Loc::Implicit,
                ty.storage_array_elem(),
                Type::StorageRef(false, Box::new(ty.clone())),
                Box::new(map),
                Box::new(Expression::Variable(pt::Loc::Implicit, arg_ty, arg_no)),
            );

            params.push(Parameter {
                id: Some(id),
                loc: pt::Loc::Implicit,
                ty: key.as_ref().clone(),
                ty_loc: None,
                indexed: false,
                readonly: false,
                recursive: false,
            });

            collect_parameters(value, symtable, params, expr, ns)
        }
        Type::Array(elem_ty, dims) => {
            let mut ty = Type::StorageRef(false, Box::new(ty.clone()));
            for _ in 0..dims.len() {
                let map = (*expr).clone();

                let id = pt::Identifier {
                    loc: pt::Loc::Implicit,
                    name: "".to_owned(),
                };
                let arg_ty = Type::Uint(256);

                let var_no = symtable
                    .add(
                        &id,
                        arg_ty.clone(),
                        ns,
                        VariableInitializer::Solidity(None),
                        VariableUsage::Parameter,
                        None,
                    )
                    .unwrap();

                symtable.arguments.push(Some(var_no));

                *expr = Expression::Subscript(
                    pt::Loc::Implicit,
                    ty.storage_array_elem(),
                    ty.clone(),
                    Box::new(map),
                    Box::new(Expression::Variable(
                        pt::Loc::Implicit,
                        Type::Uint(256),
                        var_no,
                    )),
                );

                ty = ty.storage_array_elem();

                params.push(Parameter {
                    id: Some(id),
                    loc: pt::Loc::Implicit,
                    ty: arg_ty,
                    ty_loc: None,
                    indexed: false,
                    readonly: false,
                    recursive: false,
                });
            }

            collect_parameters(elem_ty, symtable, params, expr, ns)
        }
        _ => ty,
    }
}

pub fn resolve_initializers(
    initializers: &[DelayedResolveInitializer],
    file_no: usize,
    ns: &mut Namespace,
) {
    let mut symtable = Symtable::new();
    let mut diagnostics = Vec::new();

    for DelayedResolveInitializer {
        var_no,
        contract_no,
        initializer,
    } in initializers
    {
        let var = &ns.contracts[*contract_no].variables[*var_no];
        let ty = var.ty.clone();

        let context = ExprContext {
            file_no,
            unchecked: false,
            contract_no: Some(*contract_no),
            function_no: None,
            constant: false,
            lvalue: false,
            yul_function: false,
        };

        if let Ok(res) = expression(
            initializer,
            &context,
            ns,
            &mut symtable,
            &mut diagnostics,
            ResolveTo::Type(&ty),
        ) {
            if let Ok(res) = res.cast(&initializer.loc(), &ty, true, ns, &mut diagnostics) {
                ns.contracts[*contract_no].variables[*var_no].initializer = Some(res);
            }
        }
    }

    ns.diagnostics.extend(diagnostics);
}
