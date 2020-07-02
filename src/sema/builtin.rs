use crate::Target;
use output::Output;
use parser::pt;
use sema::ast::{Builtin, Expression, Namespace, Type};
use sema::expression::{cast, expression};
use sema::symtable::Symtable;

struct Prototype {
    pub builtin: Builtin,
    pub this: Option<Type>,
    pub namespace: Option<&'static str>,
    pub name: &'static str,
    pub args: &'static [Type],
    pub ret: &'static [Type],
    pub target: Option<Target>,
}

// A list of all Solidity builtins
static PROTO_TYPES: [Prototype; 17] = [
    Prototype {
        builtin: Builtin::Assert,
        namespace: None,
        name: "assert",
        this: None,
        args: &[Type::Bool],
        ret: &[Type::Void],
        target: None,
    },
    Prototype {
        builtin: Builtin::Print,
        namespace: None,
        name: "print",
        this: None,
        args: &[Type::String],
        ret: &[Type::Void],
        target: None,
    },
    Prototype {
        builtin: Builtin::Require,
        namespace: None,
        name: "require",
        this: None,
        args: &[Type::Bool],
        ret: &[Type::Void],
        target: None,
    },
    Prototype {
        builtin: Builtin::Require,
        namespace: None,
        name: "require",
        this: None,
        args: &[Type::Bool, Type::String],
        ret: &[Type::Void],
        target: None,
    },
    Prototype {
        builtin: Builtin::Revert,
        namespace: None,
        name: "revert",
        this: None,
        args: &[],
        ret: &[Type::Unreachable],
        target: None,
    },
    Prototype {
        builtin: Builtin::Revert,
        namespace: None,
        name: "revert",
        this: None,
        args: &[Type::String],
        ret: &[Type::Unreachable],
        target: None,
    },
    Prototype {
        builtin: Builtin::SelfDestruct,
        namespace: None,
        name: "selfdestruct",
        this: None,
        args: &[Type::Address(true)],
        ret: &[Type::Unreachable],
        target: None,
    },
    Prototype {
        builtin: Builtin::Keccak256,
        namespace: None,
        name: "keccak256",
        this: None,
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Ripemd160,
        namespace: None,
        name: "ripemd160",
        this: None,
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(20)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Sha256,
        namespace: None,
        name: "sha256",
        this: None,
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Blake2_128,
        namespace: None,
        name: "blake2_128",
        this: None,
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(16)],
        target: Some(Target::Substrate),
    },
    Prototype {
        builtin: Builtin::Blake2_256,
        namespace: None,
        name: "blake2_256",
        this: None,
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: Some(Target::Substrate),
    },
    Prototype {
        builtin: Builtin::AbiDecode,
        namespace: Some("abi"),
        name: "decode",
        this: None,
        args: &[Type::DynamicBytes],
        ret: &[],
        target: None,
    },
    Prototype {
        builtin: Builtin::AbiEncode,
        namespace: Some("abi"),
        name: "encode",
        this: None,
        args: &[],
        ret: &[],
        target: None,
    },
    Prototype {
        builtin: Builtin::AbiEncodePacked,
        namespace: Some("abi"),
        name: "encodePacked",
        this: None,
        args: &[],
        ret: &[],
        target: None,
    },
    Prototype {
        builtin: Builtin::AbiEncodeWithSelector,
        namespace: Some("abi"),
        name: "encodeWithSelector",
        this: None,
        args: &[Type::Bytes(4)],
        ret: &[],
        target: None,
    },
    Prototype {
        builtin: Builtin::AbiEncodeWithSignature,
        namespace: Some("abi"),
        name: "encodeWithSignature",
        this: None,
        args: &[Type::String],
        ret: &[],
        target: None,
    },
];

/// Does function call match builtin
pub fn is_builtin_call(namespace: Option<&str>, fname: &str) -> bool {
    PROTO_TYPES
        .iter()
        .any(|p| p.name == fname && p.this.is_none() && p.namespace == namespace)
}

/// Is name reserved for builtins
pub fn is_reserved(fname: &str) -> bool {
    if fname == "type" {
        return true;
    }

    PROTO_TYPES
        .iter()
        .any(|p| (p.name == fname && p.namespace == None) || (p.namespace == Some(fname)))
}

/// Resolve a builtin call
pub fn resolve_call(
    loc: &pt::Loc,
    id: &pt::Identifier,
    args: Vec<Expression>,
    ns: &mut Namespace,
) -> Result<Expression, ()> {
    let matches = PROTO_TYPES
        .iter()
        .filter(|p| p.name == id.name && p.this.is_none() && p.namespace.is_none())
        .collect::<Vec<&Prototype>>();
    let marker = ns.diagnostics.len();
    for func in &matches {
        if func.args.len() != args.len() {
            ns.diagnostics.push(Output::error(
                *loc,
                format!(
                    "builtin function ‘{}’ expects {} arguments, {} provided",
                    func.name,
                    func.args.len(),
                    args.len()
                ),
            ));
            continue;
        }

        let mut matches = true;
        let mut cast_args = Vec::new();

        // check if arguments can be implicitly casted
        for (i, arg) in args.iter().enumerate() {
            match cast(&pt::Loc(0, 0), arg.clone(), &func.args[i], true, ns) {
                Ok(expr) => cast_args.push(expr),
                Err(()) => {
                    matches = false;
                    break;
                }
            }
        }

        if matches {
            ns.diagnostics.truncate(marker);
            return Ok(Expression::Builtin(
                *loc,
                func.ret.to_vec(),
                func.builtin.clone(),
                cast_args,
            ));
        }
    }

    if matches.len() != 1 {
        ns.diagnostics.truncate(marker);
        ns.diagnostics.push(Output::error(
            *loc,
            "cannot find overloaded function which matches signature".to_string(),
        ));
    }

    Err(())
}

/// Resolve a builtin method call. The takes the unresolved arguments, since it has
/// to handle the special case "abi.decode(foo, (int32, bool, address))" where the
/// second argument is a type list. The generic expression resolver cannot deal with
/// this. It is only used in for this specific call.
pub fn resolve_method_call(
    loc: &pt::Loc,
    namespace: &pt::Identifier,
    id: &pt::Identifier,
    args: &[pt::Expression],
    contract_no: Option<usize>,
    ns: &mut Namespace,
    symtable: &Symtable,
) -> Result<Expression, ()> {
    assert_eq!(namespace.name, "abi");

    let builtin = match id.name.as_str() {
        "decode" => Builtin::AbiDecode,
        "encode" => Builtin::AbiEncode,
        "encodePacked" => Builtin::AbiEncodePacked,
        "encodeWithSelector" => Builtin::AbiEncodeWithSelector,
        "encodeWithSignature" => Builtin::AbiEncodeWithSignature,
        _ => unreachable!(),
    };

    if builtin == Builtin::AbiDecode {
        if args.len() != 2 {
            ns.diagnostics.push(Output::error(
                *loc,
                format!("function expects {} arguments, {} provided", 2, args.len()),
            ));

            return Err(());
        }

        // first args
        let data = cast(
            &args[0].loc(),
            expression(&args[0], contract_no, ns, symtable, false)?,
            &Type::DynamicBytes,
            true,
            ns,
        )?;

        let mut tys = Vec::new();
        let mut broken = false;

        match &args[1] {
            pt::Expression::List(_, list) => {
                for (loc, param) in list {
                    if let Some(param) = param {
                        let ty = ns.resolve_type(contract_no, false, &param.ty)?;

                        if let Some(storage) = &param.storage {
                            ns.diagnostics.push(Output::error(
                                *storage.loc(),
                                format!("storage modifier ‘{}’ not allowed", storage),
                            ));
                            broken = true;
                        }

                        if let Some(name) = &param.name {
                            ns.diagnostics.push(Output::error(
                                name.loc,
                                format!("unexpected identifier ‘{}’ in type", name.name),
                            ));
                            broken = true;
                        }

                        if ty.is_mapping() {
                            ns.diagnostics.push(Output::error(
                                *loc,
                                "mapping cannot be abi decoded or encoded".to_string(),
                            ));
                            broken = true;
                        }

                        tys.push(ty);
                    } else {
                        ns.diagnostics
                            .push(Output::error(*loc, "missing type".to_string()));

                        broken = true;
                    }
                }
            }
            _ => {
                let ty = ns.resolve_type(contract_no, false, &args[1])?;

                if ty.is_mapping() {
                    ns.diagnostics.push(Output::error(
                        *loc,
                        "mapping cannot be abi decoded or encoded".to_string(),
                    ));
                    broken = true;
                }

                tys.push(ty);
            }
        }

        return if broken {
            Err(())
        } else {
            Ok(Expression::Builtin(
                *loc,
                tys,
                Builtin::AbiDecode,
                vec![data],
            ))
        };
    }

    let mut resolved_args = Vec::new();
    let mut args_iter = args.iter();

    match builtin {
        Builtin::AbiEncodeWithSelector => {
            // first argument is selector
            if let Some(selector) = args_iter.next() {
                let selector = expression(selector, contract_no, ns, symtable, false)?;

                resolved_args.insert(
                    0,
                    cast(&selector.loc(), selector, &Type::Bytes(4), true, ns)?,
                );
            } else {
                ns.diagnostics.push(Output::error(
                    *loc,
                    "function requires one ‘bytes4’ selector argument".to_string(),
                ));

                return Err(());
            }
        }
        Builtin::AbiEncodeWithSignature => {
            // first argument is signature
            if let Some(signature) = args_iter.next() {
                let signature = expression(signature, contract_no, ns, symtable, false)?;

                resolved_args.insert(
                    0,
                    cast(&signature.loc(), signature, &Type::String, true, ns)?,
                );
            } else {
                ns.diagnostics.push(Output::error(
                    *loc,
                    "function requires one ‘string’ signature argument".to_string(),
                ));

                return Err(());
            }
        }
        _ => (),
    }

    for arg in args_iter {
        let mut expr = expression(arg, contract_no, ns, symtable, false)?;
        let ty = expr.ty();

        if ty.is_mapping() {
            ns.diagnostics.push(Output::error(
                arg.loc(),
                "mapping type not permitted".to_string(),
            ));

            return Err(());
        }

        expr = cast(&arg.loc(), expr, ty.deref_any(), true, ns)?;

        // A string or hex literal should be encoded as a string
        if let Expression::BytesLiteral(_, _, _) = &expr {
            expr = cast(&arg.loc(), expr, &Type::String, true, ns)?;
        }

        resolved_args.push(expr);
    }

    Ok(Expression::Builtin(
        *loc,
        vec![Type::DynamicBytes],
        builtin,
        resolved_args,
    ))
}
