use super::{FunctionDecl, Namespace, Parameter, Symbol, Type};
use output::Output;
use parser::ast;
use Target;

pub fn function_decl(
    f: &ast::FunctionDefinition,
    i: usize,
    contract_no: usize,
    ns: &mut Namespace,
    errors: &mut Vec<Output>,
) -> bool {
    let mut success = true;

    // The parser allows constructors to have return values. This is so that we can give a
    // nicer error message than "returns unexpected"
    match f.ty {
        ast::FunctionTy::Function => {
            // Function name cannot be the same as the contract name
            if let Some(n) = &f.name {
                if n.name == ns.contracts[contract_no].name {
                    errors.push(Output::error(
                        f.loc,
                        "function cannot have same name as the contract".to_string(),
                    ));
                    return false;
                }
            } else {
                errors.push(Output::error(
                    f.name_loc,
                    "function is missing a name. did you mean ‘fallback() extern {…}’ or ‘receive() extern {…}’?".to_string(),
                ));
                return false;
            }
        }
        ast::FunctionTy::Constructor => {
            if !f.returns.is_empty() {
                errors.push(Output::warning(
                    f.loc,
                    "constructor cannot have return values".to_string(),
                ));
                return false;
            }
            if f.name.is_some() {
                errors.push(Output::warning(
                    f.loc,
                    "constructor cannot have a name".to_string(),
                ));
                return false;
            }
        }
        ast::FunctionTy::Fallback | ast::FunctionTy::Receive => {
            if !f.returns.is_empty() {
                errors.push(Output::warning(
                    f.loc,
                    format!("{} function cannot have return values", f.ty),
                ));
                success = false;
            }
            if !f.params.is_empty() {
                errors.push(Output::warning(
                    f.loc,
                    format!("{} function cannot have parameters", f.ty),
                ));
                success = false;
            }
            if f.name.is_some() {
                errors.push(Output::warning(
                    f.loc,
                    format!("{} function cannot have a name", f.ty),
                ));
                return false;
            }
        }
    }

    let mut mutability: Option<ast::StateMutability> = None;
    let mut visibility: Option<ast::Visibility> = None;

    for a in &f.attributes {
        match &a {
            ast::FunctionAttribute::StateMutability(m) => {
                if let Some(e) = &mutability {
                    errors.push(Output::error_with_note(
                        m.loc(),
                        format!("function redeclared `{}'", m.to_string()),
                        e.loc(),
                        format!("location of previous declaration of `{}'", e.to_string()),
                    ));
                    success = false;
                    continue;
                }

                mutability = Some(m.clone());
            }
            ast::FunctionAttribute::Visibility(v) => {
                if let Some(e) = &visibility {
                    errors.push(Output::error_with_note(
                        v.loc(),
                        format!("function redeclared `{}'", v.to_string()),
                        e.loc(),
                        format!("location of previous declaration of `{}'", e.to_string()),
                    ));
                    success = false;
                    continue;
                }

                visibility = Some(v.clone());
            }
        }
    }

    let visibility = match visibility {
        Some(v) => v,
        None => {
            errors.push(Output::error(f.loc, "no visibility specified".to_string()));
            success = false;
            // continue processing while assuming it's a public
            ast::Visibility::Public(ast::Loc(0, 0))
        }
    };

    // Reference types can't be passed through the ABI encoder/decoder, so
    // storage parameters/returns are only allowed in internal/private functions
    let storage_allowed = match visibility {
        ast::Visibility::Internal(_) | ast::Visibility::Private(_) => {
            if let Some(ast::StateMutability::Payable(loc)) = mutability {
                errors.push(Output::error(
                    loc,
                    "internal or private function cannot be payable".to_string(),
                ));
                success = false;
            }
            true
        }
        ast::Visibility::Public(_) | ast::Visibility::External(_) => false,
    };

    let (params, params_success) = resolve_params(f, storage_allowed, contract_no, ns, errors);

    let (returns, returns_success) = resolve_returns(f, storage_allowed, contract_no, ns, errors);

    if !success || !returns_success || !params_success {
        return false;
    }

    let name = match &f.name {
        Some(s) => s.name.to_owned(),
        None => "".to_owned(),
    };

    let fdecl = FunctionDecl::new(
        f.loc,
        name,
        f.doc.clone(),
        f.ty.clone(),
        Some(i),
        mutability,
        visibility,
        params,
        returns,
        ns,
    );

    if f.ty == ast::FunctionTy::Constructor {
        // In the eth solidity, only one constructor is allowed
        if ns.target == Target::Ewasm {
            if let Some(prev) = ns.contracts[contract_no]
                .functions
                .iter()
                .find(|f| f.is_constructor())
            {
                errors.push(Output::error_with_note(
                    f.loc,
                    "constructor already defined".to_string(),
                    prev.loc,
                    "location of previous definition".to_string(),
                ));
                return false;
            }
        } else {
            let payable = fdecl.is_payable();

            if let Some(prev) = ns.contracts[contract_no]
                .functions
                .iter()
                .find(|f| f.is_constructor() && f.is_payable() != payable)
            {
                errors.push(Output::error_with_note(
                    f.loc,
                    "all constructors should be defined ‘payable’ or not".to_string(),
                    prev.loc,
                    "location of previous definition".to_string(),
                ));
                return false;
            }
        }

        // FIXME: Internal visibility is allowed on abstract contracts, but we don't support those yet
        match fdecl.visibility {
            ast::Visibility::Public(_) => (),
            _ => {
                errors.push(Output::error(
                    f.loc,
                    "constructor function must be declared public".to_owned(),
                ));
                return false;
            }
        }

        match fdecl.mutability {
            Some(ast::StateMutability::Pure(loc)) => {
                errors.push(Output::error(
                    loc,
                    "constructor cannot be declared pure".to_string(),
                ));
                return false;
            }
            Some(ast::StateMutability::View(loc)) => {
                errors.push(Output::error(
                    loc,
                    "constructor cannot be declared view".to_string(),
                ));
                return false;
            }
            _ => (),
        }

        for v in ns.contracts[contract_no]
            .functions
            .iter()
            .filter(|f| f.is_constructor())
        {
            if v.signature == fdecl.signature {
                errors.push(Output::error_with_note(
                    f.loc,
                    "constructor with this signature already exists".to_string(),
                    v.loc,
                    "location of previous definition".to_string(),
                ));

                return false;
            }
        }

        ns.contracts[contract_no].functions.push(fdecl);

        true
    } else if f.ty == ast::FunctionTy::Receive || f.ty == ast::FunctionTy::Fallback {
        if let Some(prev) = ns.contracts[contract_no]
            .functions
            .iter()
            .find(|o| o.ty == f.ty)
        {
            errors.push(Output::error_with_note(
                f.loc,
                format!("{} function already defined", f.ty),
                prev.loc,
                "location of previous definition".to_string(),
            ));
            return false;
        }

        if let ast::Visibility::External(_) = fdecl.visibility {
            // ok
        } else {
            errors.push(Output::error(
                f.loc,
                format!("{} function must be declared external", f.ty),
            ));
            return false;
        }

        if let Some(ast::StateMutability::Payable(_)) = fdecl.mutability {
            if f.ty == ast::FunctionTy::Fallback {
                errors.push(Output::error(
                    f.loc,
                    format!("{} function must not be declare payable, use ‘receive() external payable’ instead", f.ty),
                ));
                return false;
            }
        } else if f.ty == ast::FunctionTy::Receive {
            errors.push(Output::error(
                f.loc,
                format!("{} function must be declared payable", f.ty),
            ));
            return false;
        }

        ns.contracts[contract_no].functions.push(fdecl);

        true
    } else {
        let id = f.name.as_ref().unwrap();

        if let Some(Symbol::Function(ref mut v)) =
            ns.symbols.get_mut(&(Some(contract_no), id.name.to_owned()))
        {
            // check if signature already present
            for o in v.iter() {
                if ns.contracts[contract_no].functions[o.1].signature == fdecl.signature {
                    errors.push(Output::error_with_note(
                        f.loc,
                        "overloaded function with this signature already exist".to_string(),
                        o.0,
                        "location of previous definition".to_string(),
                    ));
                    return false;
                }
            }

            let pos = ns.contracts[contract_no].functions.len();

            ns.contracts[contract_no].functions.push(fdecl);

            v.push((f.loc, pos));
            return true;
        }

        let pos = ns.contracts[contract_no].functions.len();

        ns.contracts[contract_no].functions.push(fdecl);

        ns.add_symbol(
            Some(contract_no),
            id,
            Symbol::Function(vec![(id.loc, pos)]),
            errors,
        );

        true
    }
}

/// Resolve the parameters
fn resolve_params(
    f: &ast::FunctionDefinition,
    storage_allowed: bool,
    contract_no: usize,
    ns: &mut Namespace,
    errors: &mut Vec<Output>,
) -> (Vec<Parameter>, bool) {
    let mut params = Vec::new();
    let mut success = true;

    for p in &f.params {
        let p = match p {
            (_, Some(p)) => p,
            (loc, None) => {
                errors.push(Output::error(*loc, "missing parameter type".to_owned()));
                success = false;
                continue;
            }
        };

        match ns.resolve_type(Some(contract_no), false, &p.ty, errors) {
            Ok(ty) => {
                let ty = if !ty.can_have_data_location() {
                    if let Some(storage) = &p.storage {
                        errors.push(Output::error(
                                *storage.loc(),
                                format!("data location ‘{}’ can only be specified for array, struct or mapping",
                                storage)
                            ));
                        success = false;
                    }

                    ty
                } else if let Some(ast::StorageLocation::Storage(loc)) = p.storage {
                    if storage_allowed {
                        Type::StorageRef(Box::new(ty))
                    } else {
                        errors.push(Output::error(
                            loc,
                            "parameter of type ‘storage’ not allowed public or external functions"
                                .to_string(),
                        ));
                        success = false;
                        ty
                    }
                } else if ty.contains_mapping(ns) {
                    errors.push(Output::error(
                        p.ty.loc(),
                        "parameter with mapping type must be of type ‘storage’".to_string(),
                    ));
                    success = false;
                    ty
                } else {
                    ty
                };

                params.push(Parameter {
                    name: p
                        .name
                        .as_ref()
                        .map_or("".to_string(), |id| id.name.to_string()),
                    ty,
                });
            }
            Err(()) => success = false,
        }
    }

    (params, success)
}

/// Resolve the return values
fn resolve_returns(
    f: &ast::FunctionDefinition,
    storage_allowed: bool,
    contract_no: usize,
    ns: &mut Namespace,
    errors: &mut Vec<Output>,
) -> (Vec<Parameter>, bool) {
    let mut returns = Vec::new();
    let mut success = true;

    for r in &f.returns {
        let r = match r {
            (_, Some(p)) => p,
            (loc, None) => {
                errors.push(Output::error(*loc, "missing return type".to_owned()));
                success = false;
                continue;
            }
        };

        match ns.resolve_type(Some(contract_no), false, &r.ty, errors) {
            Ok(ty) => {
                let ty = if !ty.can_have_data_location() {
                    if let Some(storage) = &r.storage {
                        errors.push(Output::error(
                                *storage.loc(),
                                format!("data location ‘{}’ can only be specified for array, struct or mapping",
                                storage)
                            ));
                        success = false;
                    }

                    ty
                } else {
                    match r.storage {
                        Some(ast::StorageLocation::Calldata(loc)) => {
                            errors.push(Output::error(
                                loc,
                                "data location ‘calldata’ can not be used for return types"
                                    .to_string(),
                            ));
                            success = false;
                            ty
                        }
                        Some(ast::StorageLocation::Storage(loc)) => {
                            if storage_allowed {
                                Type::StorageRef(Box::new(ty))
                            } else {
                                errors.push(Output::error(
                                    loc,
                                    "return type of type ‘storage’ not allowed public or external functions"
                                        .to_string(),
                                ));
                                success = false;
                                ty
                            }
                        }
                        _ => {
                            if ty.contains_mapping(ns) {
                                errors.push(Output::error(
                                    r.ty.loc(),
                                    "return type containing mapping must be of type ‘storage’"
                                        .to_string(),
                                ));
                                success = false;
                            }

                            ty
                        }
                    }
                };

                returns.push(Parameter {
                    name: r
                        .name
                        .as_ref()
                        .map_or("".to_string(), |id| id.name.to_string()),
                    ty,
                });
            }
            Err(()) => success = false,
        }
    }

    (returns, success)
}

#[test]
fn signatures() {
    use super::*;

    let ns = Namespace::new(Target::Ewasm, 20);

    let fdecl = FunctionDecl::new(
        ast::Loc(0, 0),
        "foo".to_owned(),
        vec![],
        ast::FunctionTy::Function,
        Some(0),
        None,
        ast::Visibility::Public(ast::Loc(0, 0)),
        vec![
            Parameter {
                name: "".to_string(),
                ty: Type::Uint(8),
            },
            Parameter {
                name: "".to_string(),
                ty: Type::Address(false),
            },
        ],
        Vec::new(),
        &ns,
    );

    assert_eq!(fdecl.signature, "foo(uint8,address)");
}
