use parser::ast;
use output::Output;
use Target;
use super::{Contract, Parameter, FunctionDecl, Symbol};

pub fn function_decl(
    f: &ast::FunctionDefinition,
    i: usize,
    ns: &mut Contract,
    errors: &mut Vec<Output>,
) -> bool {
    let mut params = Vec::new();
    let mut returns = Vec::new();
    let mut success = true;

    // The parser allows constructors to have return values. This is so that we can give a
    // nicer error message than "returns unexpected"
    if f.constructor && !f.returns.is_empty() {
        errors.push(Output::warning(
            f.loc,
            format!("constructor cannot have return values"),
        ));
        return false;
    } else if !f.constructor && f.name == None {
        if !f.returns.is_empty() {
            errors.push(Output::warning(
                f.loc,
                format!("fallback function cannot have return values"),
            ));
            success = false;
        }

        if !f.params.is_empty() {
            errors.push(Output::warning(
                f.loc,
                format!("fallback function cannot have parameters"),
            ));
            success = false;
        }
    }

    for p in &f.params {
        match ns.resolve_type(&p.typ, errors) {
            Ok(s) => params.push(Parameter {
                name: p
                    .name
                    .as_ref()
                    .map_or("".to_string(), |id| id.name.to_string()),
                ty: s,
            }),
            Err(()) => success = false,
        }
    }

    for r in &f.returns {
        // FIXME: these should be allowed
        if let Some(ref n) = r.name {
            errors.push(Output::warning(
                n.loc,
                format!("named return value `{}' not allowed", n.name),
            ));
        }

        match ns.resolve_type(&r.typ, errors) {
            Ok(s) => returns.push(Parameter {
                name: r
                    .name
                    .as_ref()
                    .map_or("".to_string(), |id| id.name.to_string()),
                ty: s,
            }),
            Err(()) => success = false,
        }
    }

    let mut mutability: Option<ast::StateMutability> = None;
    let mut visibility: Option<ast::Visibility> = None;

    for a in &f.attributes {
        match &a {
            ast::FunctionAttribute::StateMutability(m) => {
                if let Some(e) = &mutability {
                    errors.push(Output::error_with_note(
                        m.loc().clone(),
                        format!("function redeclared `{}'", m.to_string()),
                        e.loc().clone(),
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
                        v.loc().clone(),
                        format!("function redeclared `{}'", v.to_string()),
                        e.loc().clone(),
                        format!("location of previous declaration of `{}'", e.to_string()),
                    ));
                    success = false;
                    continue;
                }

                visibility = Some(v.clone());
            }
        }
    }

    if visibility == None {
        errors.push(Output::error(
            f.loc,
            format!("no visibility specified"),
        ));
        success = false;
    }

    if !success {
        return false;
    }

    let (name, fallback) = match f.name {
        Some(ref n) => (n.name.to_owned(), false),
        None => ("".to_owned(), !f.constructor),
    };

    let fdecl = FunctionDecl::new(f.loc, name, fallback, i, mutability, visibility.unwrap(), params, returns, &ns);

    if f.constructor {
        // In the eth solidity, only one constructor is allowed
        if ns.target == Target::Burrow && !ns.constructors.is_empty() {
            let prev = &ns.constructors[i];
            errors.push(Output::error_with_note(
                f.loc,
                "constructor already defined".to_string(),
                prev.loc,
                "location of previous definition".to_string(),
            ));
            return false;
        }

        // FIXME: Internal visibility is allowed on abstract contracts, but we don't support those yet
        match fdecl.visibility {
            ast::Visibility::Public(_) => (),
            _ => {
                errors.push(Output::error(
                    f.loc,
                    "constructor function must be declared public".to_owned()
                ));
                return false;
            }
        }

        match fdecl.mutability {
            Some(ast::StateMutability::Pure(loc)) => {
                errors.push(Output::error(
                    loc,
                    format!("constructor cannot be declared pure"),
                ));
                return false;
            },
            Some(ast::StateMutability::View(loc)) => {
                errors.push(Output::error(
                    loc,
                    format!("constructor cannot be declared view"),
                ));
                return false;
            },
            _ => ()
        }

        for v in ns.constructors.iter() {
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

        ns.constructors.push(fdecl);

        true
    } else if let Some(ref id) = f.name {
        if let Some(Symbol::Function(ref mut v)) = ns.symbols.get_mut(&id.name) {
            // check if signature already present
            for o in v.iter() {
                if ns.functions[o.1].signature == fdecl.signature {
                    errors.push(Output::error_with_note(
                        f.loc,
                        "overloaded function with this signature already exist".to_string(),
                        o.0.clone(),
                        "location of previous definition".to_string(),
                    ));
                    return false;
                }
            }

            let pos = ns.functions.len();

            ns.functions.push(fdecl);

            v.push((f.loc, pos));
            return true;
        }

        let pos = ns.functions.len();

        ns.functions.push(fdecl);

        ns.add_symbol(id, Symbol::Function(vec![(id.loc, pos)]), errors)
    } else {
        // fallback function
        if let Some(i) = ns.fallback_function() {
            let prev = &ns.functions[i];

            errors.push(Output::error_with_note(
                f.loc,
                "fallback function already defined".to_string(),
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
                "fallback function must be declared external".to_owned()
            ));
            return false;
        }

        ns.functions.push(fdecl);

        true
    }
}

#[test]
fn signatures() {
    use super::*;

    let ns = Contract {
        name: String::from("foo"),
        enums: Vec::new(),
        constructors: Vec::new(),
        functions: Vec::new(),
        variables: Vec::new(),
        constants: Vec::new(),
        initializer: cfg::ControlFlowGraph::new(),
        target: crate::Target::Burrow,
        top_of_contract_storage: 0,
        symbols: HashMap::new(),
    };

    let fdecl = FunctionDecl::new(
        ast::Loc(0, 0), "foo".to_owned(), false, 0, None, ast::Visibility::Public(ast::Loc(0, 0)),
        vec!(
            Parameter {
                name: "".to_string(),
                ty: Type::Primitive(ast::PrimitiveType::Uint(8))
            },
            Parameter {
                name: "".to_string(),
                ty: Type::Primitive(ast::PrimitiveType::Address)
            },
        ), Vec::new(), &ns);

    assert_eq!(fdecl.signature, "foo(uint8,address)");
}
