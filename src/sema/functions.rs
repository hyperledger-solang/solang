use super::ast::{Diagnostic, Function, Namespace, Parameter, Symbol, Type};
use parser::pt;
use Target;

/// Resolve function declaration
pub fn function_decl(
    func: &pt::FunctionDefinition,
    file_no: usize,
    contract_no: usize,
    ns: &mut Namespace,
) -> Option<usize> {
    let mut success = true;

    // The parser allows constructors to have return values. This is so that we can give a
    // nicer error message than "returns unexpected"
    match func.ty {
        pt::FunctionTy::Function => {
            // Function name cannot be the same as the contract name
            if let Some(n) = &func.name {
                if n.name == ns.contracts[contract_no].name {
                    ns.diagnostics.push(Diagnostic::error(
                        func.loc,
                        "function cannot have same name as the contract".to_string(),
                    ));
                    return None;
                }
            } else {
                ns.diagnostics.push(Diagnostic::error(
                    func.name_loc,
                    "function is missing a name. did you mean ‘fallback() extern {…}’ or ‘receive() extern {…}’?".to_string(),
                ));
                return None;
            }
        }
        pt::FunctionTy::Constructor => {
            if !func.returns.is_empty() {
                ns.diagnostics.push(Diagnostic::error(
                    func.loc,
                    "constructor cannot have return values".to_string(),
                ));
                return None;
            }
            if func.name.is_some() {
                ns.diagnostics.push(Diagnostic::error(
                    func.loc,
                    "constructor cannot have a name".to_string(),
                ));
                return None;
            }
        }
        pt::FunctionTy::Fallback | pt::FunctionTy::Receive => {
            if !func.returns.is_empty() {
                ns.diagnostics.push(Diagnostic::error(
                    func.loc,
                    format!("{} function cannot have return values", func.ty),
                ));
                success = false;
            }
            if !func.params.is_empty() {
                ns.diagnostics.push(Diagnostic::error(
                    func.loc,
                    format!("{} function cannot have parameters", func.ty),
                ));
                success = false;
            }
            if func.name.is_some() {
                ns.diagnostics.push(Diagnostic::error(
                    func.loc,
                    format!("{} function cannot have a name", func.ty),
                ));
                return None;
            }
        }
    }

    let mut mutability: Option<pt::StateMutability> = None;
    let mut visibility: Option<pt::Visibility> = None;
    let mut is_virtual: Option<pt::Loc> = None;
    let mut is_override: Option<(pt::Loc, Vec<usize>)> = None;

    for a in &func.attributes {
        match &a {
            pt::FunctionAttribute::StateMutability(m) => {
                if let Some(e) = &mutability {
                    ns.diagnostics.push(Diagnostic::error_with_note(
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
            pt::FunctionAttribute::Visibility(v) => {
                if let Some(e) = &visibility {
                    ns.diagnostics.push(Diagnostic::error_with_note(
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
            pt::FunctionAttribute::Virtual(loc) => {
                if let Some(prev_loc) = &is_virtual {
                    ns.diagnostics.push(Diagnostic::error_with_note(
                        *loc,
                        "function redeclared ‘virtual’".to_string(),
                        *prev_loc,
                        "location of previous declaration of ‘virtual’".to_string(),
                    ));
                    success = false;
                    continue;
                }

                is_virtual = Some(*loc);
            }
            pt::FunctionAttribute::Override(loc, bases) => {
                if let Some((prev_loc, _)) = &is_override {
                    ns.diagnostics.push(Diagnostic::error_with_note(
                        *loc,
                        "function redeclared ‘override’".to_string(),
                        *prev_loc,
                        "location of previous declaration of ‘override’".to_string(),
                    ));
                    success = false;
                    continue;
                }

                let mut list = Vec::new();

                for name in bases {
                    match ns.resolve_contract(file_no, name) {
                        Some(no) => {
                            // check override is base contract of our contract
                            fn is_base(no: &usize, contract_no: usize, ns: &Namespace) -> bool {
                                let inherits = &ns.contracts[contract_no].inherit;

                                if inherits.contains(no) {
                                    return true;
                                }

                                inherits
                                    .iter()
                                    .any(|contract_no| is_base(no, *contract_no, ns))
                            }

                            if list.contains(&no) {
                                ns.diagnostics.push(Diagnostic::error(
                                    name.loc,
                                    format!("function duplicate override ‘{}’", name.name),
                                ));
                            } else if !is_base(&no, contract_no, ns) {
                                ns.diagnostics.push(Diagnostic::error(
                                    name.loc,
                                    format!(
                                        "override ‘{}’ is not a base contract of ‘{}’",
                                        name.name, ns.contracts[contract_no].name
                                    ),
                                ));
                            } else {
                                list.push(no);
                            }
                        }
                        None => {
                            ns.diagnostics.push(Diagnostic::error(
                                name.loc,
                                format!("contract ‘{}’ in override list not found", name.name),
                            ));
                        }
                    }
                }

                is_override = Some((*loc, list));
            }
        }
    }

    let visibility = match visibility {
        Some(v) => v,
        None => {
            ns.diagnostics.push(Diagnostic::error(
                func.loc,
                "no visibility specified".to_string(),
            ));
            success = false;
            // continue processing while assuming it's a public
            pt::Visibility::Public(pt::Loc(0, 0, 0))
        }
    };

    // Reference types can't be passed through the ABI encoder/decoder, so
    // storage parameters/returns are only allowed in internal/private functions
    let storage_allowed = match visibility {
        pt::Visibility::Internal(_) | pt::Visibility::Private(_) => {
            if let Some(pt::StateMutability::Payable(loc)) = mutability {
                ns.diagnostics.push(Diagnostic::error(
                    loc,
                    "internal or private function cannot be payable".to_string(),
                ));
                success = false;
            }
            true
        }
        pt::Visibility::Public(_) | pt::Visibility::External(_) => false,
    };

    let (params, params_success) = resolve_params(func, storage_allowed, file_no, contract_no, ns);

    let (returns, returns_success) =
        resolve_returns(func, storage_allowed, file_no, contract_no, ns);

    if is_virtual.is_none() && func.body.is_empty() {
        ns.diagnostics.push(Diagnostic::error(
            func.loc,
            "function with no body must be marked ‘virtual’".to_string(),
        ));
        success = false;
    }

    if !success || !returns_success || !params_success {
        return None;
    }

    let name = match &func.name {
        Some(s) => s.name.to_owned(),
        None => "".to_owned(),
    };

    let mut fdecl = Function::new(
        func.loc,
        name,
        func.doc.clone(),
        func.ty.clone(),
        mutability,
        visibility,
        params,
        returns,
        ns,
    );

    fdecl.is_virtual = is_virtual.is_some();
    fdecl.is_override = is_override;

    if func.ty == pt::FunctionTy::Constructor {
        // In the eth solidity, only one constructor is allowed
        if ns.target == Target::Ewasm {
            if let Some(prev) = ns.contracts[contract_no]
                .functions
                .iter()
                .find(|f| f.is_constructor())
            {
                ns.diagnostics.push(Diagnostic::error_with_note(
                    func.loc,
                    "constructor already defined".to_string(),
                    prev.loc,
                    "location of previous definition".to_string(),
                ));
                return None;
            }
        } else {
            let payable = fdecl.is_payable();

            if let Some(prev) = ns.contracts[contract_no]
                .functions
                .iter()
                .find(|f| f.is_constructor() && f.is_payable() != payable)
            {
                ns.diagnostics.push(Diagnostic::error_with_note(
                    func.loc,
                    "all constructors should be defined ‘payable’ or not".to_string(),
                    prev.loc,
                    "location of previous definition".to_string(),
                ));
                return None;
            }
        }

        // FIXME: Internal visibility is allowed on abstract contracts, but we don't support those yet
        match fdecl.visibility {
            pt::Visibility::Public(_) => (),
            _ => {
                ns.diagnostics.push(Diagnostic::error(
                    func.loc,
                    "constructor function must be declared public".to_owned(),
                ));
                return None;
            }
        }

        match fdecl.mutability {
            Some(pt::StateMutability::Pure(loc)) => {
                ns.diagnostics.push(Diagnostic::error(
                    loc,
                    "constructor cannot be declared pure".to_string(),
                ));
                return None;
            }
            Some(pt::StateMutability::View(loc)) => {
                ns.diagnostics.push(Diagnostic::error(
                    loc,
                    "constructor cannot be declared view".to_string(),
                ));
                return None;
            }
            _ => (),
        }

        for v in ns.contracts[contract_no]
            .functions
            .iter()
            .filter(|f| f.is_constructor())
        {
            if v.signature == fdecl.signature {
                ns.diagnostics.push(Diagnostic::error_with_note(
                    func.loc,
                    "constructor with this signature already exists".to_string(),
                    v.loc,
                    "location of previous definition".to_string(),
                ));

                return None;
            }
        }

        let pos = ns.contracts[contract_no].functions.len();

        ns.contracts[contract_no]
            .function_table
            .insert(fdecl.signature.to_owned(), (contract_no, pos, None));

        ns.contracts[contract_no].functions.push(fdecl);

        Some(pos)
    } else if func.ty == pt::FunctionTy::Receive || func.ty == pt::FunctionTy::Fallback {
        if let Some(prev) = ns.contracts[contract_no]
            .functions
            .iter()
            .find(|o| o.ty == func.ty)
        {
            ns.diagnostics.push(Diagnostic::error_with_note(
                func.loc,
                format!("{} function already defined", func.ty),
                prev.loc,
                "location of previous definition".to_string(),
            ));
            return None;
        }

        if let pt::Visibility::External(_) = fdecl.visibility {
            // ok
        } else {
            ns.diagnostics.push(Diagnostic::error(
                func.loc,
                format!("{} function must be declared external", func.ty),
            ));
            return None;
        }

        if let Some(pt::StateMutability::Payable(_)) = fdecl.mutability {
            if func.ty == pt::FunctionTy::Fallback {
                ns.diagnostics.push(Diagnostic::error(
                    func.loc,
                    format!("{} function must not be declare payable, use ‘receive() external payable’ instead", func.ty),
                ));
                return None;
            }
        } else if func.ty == pt::FunctionTy::Receive {
            ns.diagnostics.push(Diagnostic::error(
                func.loc,
                format!("{} function must be declared payable", func.ty),
            ));
            return None;
        }

        let pos = ns.contracts[contract_no].functions.len();

        ns.contracts[contract_no]
            .function_table
            .insert(fdecl.signature.to_owned(), (contract_no, pos, None));

        ns.contracts[contract_no].functions.push(fdecl);

        Some(pos)
    } else {
        let id = func.name.as_ref().unwrap();

        if let Some((func_contract_no, func_no, _)) = ns.contracts[contract_no]
            .function_table
            .get(&fdecl.signature)
        {
            ns.diagnostics.push(Diagnostic::error_with_note(
                func.loc,
                "overloaded function with this signature already exist".to_string(),
                ns.contracts[*func_contract_no].functions[*func_no].loc,
                "location of previous definition".to_string(),
            ));

            return None;
        }

        let func_no = ns.contracts[contract_no].functions.len();

        ns.contracts[contract_no]
            .function_table
            .insert(fdecl.signature.to_owned(), (contract_no, func_no, None));

        ns.contracts[contract_no].functions.push(fdecl);

        if let Some(Symbol::Function(ref mut v)) =
            ns.symbols
                .get_mut(&(file_no, Some(contract_no), id.name.to_owned()))
        {
            v.push(func.loc);
        } else {
            ns.add_symbol(
                file_no,
                Some(contract_no),
                id,
                Symbol::Function(vec![id.loc]),
            );
        }

        Some(func_no)
    }
}

/// Resolve the parameters
fn resolve_params(
    f: &pt::FunctionDefinition,
    storage_allowed: bool,
    file_no: usize,
    contract_no: usize,
    ns: &mut Namespace,
) -> (Vec<Parameter>, bool) {
    let mut params = Vec::new();
    let mut success = true;

    for (loc, p) in &f.params {
        let p = match p {
            Some(p) => p,
            None => {
                ns.diagnostics
                    .push(Diagnostic::error(*loc, "missing parameter type".to_owned()));
                success = false;
                continue;
            }
        };

        match ns.resolve_type(file_no, Some(contract_no), false, &p.ty) {
            Ok(ty) => {
                let ty = if !ty.can_have_data_location() {
                    if let Some(storage) = &p.storage {
                        ns.diagnostics.push(Diagnostic::error(
                            *storage.loc(),
                                format!("data location ‘{}’ can only be specified for array, struct or mapping",
                                storage)
                            ));
                        success = false;
                    }

                    ty
                } else if let Some(pt::StorageLocation::Storage(loc)) = p.storage {
                    if storage_allowed {
                        Type::StorageRef(Box::new(ty))
                    } else {
                        ns.diagnostics.push(Diagnostic::error(
                            loc,
                            "parameter of type ‘storage’ not allowed public or external functions"
                                .to_string(),
                        ));
                        success = false;
                        ty
                    }
                } else if ty.contains_mapping(ns) {
                    ns.diagnostics.push(Diagnostic::error(
                        p.ty.loc(),
                        "parameter with mapping type must be of type ‘storage’".to_string(),
                    ));
                    success = false;
                    ty
                } else {
                    ty
                };

                params.push(Parameter {
                    loc: *loc,
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
    f: &pt::FunctionDefinition,
    storage_allowed: bool,
    file_no: usize,
    contract_no: usize,
    ns: &mut Namespace,
) -> (Vec<Parameter>, bool) {
    let mut returns = Vec::new();
    let mut success = true;

    for (loc, r) in &f.returns {
        let r = match r {
            Some(r) => r,
            None => {
                ns.diagnostics
                    .push(Diagnostic::error(*loc, "missing return type".to_owned()));
                success = false;
                continue;
            }
        };

        match ns.resolve_type(file_no, Some(contract_no), false, &r.ty) {
            Ok(ty) => {
                let ty = if !ty.can_have_data_location() {
                    if let Some(storage) = &r.storage {
                        ns.diagnostics.push(Diagnostic::error(
                            *storage.loc(),
                                format!("data location ‘{}’ can only be specified for array, struct or mapping",
                                storage)
                            ));
                        success = false;
                    }

                    ty
                } else {
                    match r.storage {
                        Some(pt::StorageLocation::Calldata(loc)) => {
                            ns.diagnostics.push(Diagnostic::error(
                                loc,
                                "data location ‘calldata’ can not be used for return types"
                                    .to_string(),
                            ));
                            success = false;
                            ty
                        }
                        Some(pt::StorageLocation::Storage(loc)) => {
                            if storage_allowed {
                                Type::StorageRef(Box::new(ty))
                            } else {
                                ns.diagnostics.push(Diagnostic::error(
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
                                ns.diagnostics.push(Diagnostic::error(
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
                    loc: *loc,
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

    let ns = Namespace::new(Target::Ewasm, 20, 16);

    let fdecl = Function::new(
        pt::Loc(0, 0, 0),
        "foo".to_owned(),
        vec![],
        pt::FunctionTy::Function,
        None,
        pt::Visibility::Public(pt::Loc(0, 0, 0)),
        vec![
            Parameter {
                loc: pt::Loc(0, 0, 0),
                name: "".to_string(),
                ty: Type::Uint(8),
            },
            Parameter {
                loc: pt::Loc(0, 0, 0),
                name: "".to_string(),
                ty: Type::Address(false),
            },
        ],
        Vec::new(),
        &ns,
    );

    assert_eq!(fdecl.signature, "foo(uint8,address)");
}
