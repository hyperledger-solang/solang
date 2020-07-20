use crate::Target;
use output::Diagnostic;
use parser::pt;
use sema::ast::{Builtin, Expression, Namespace, Type};
use sema::expression::{cast, expression};
use sema::symtable::Symtable;

struct Prototype {
    pub builtin: Builtin,
    pub namespace: Option<&'static str>,
    pub name: &'static str,
    pub args: &'static [Type],
    pub ret: &'static [Type],
    pub target: Option<Target>,
}

// A list of all Solidity builtins functions
static BUILTIN_FUNCTIONS: [Prototype; 22] = [
    Prototype {
        builtin: Builtin::Assert,
        namespace: None,
        name: "assert",
        args: &[Type::Bool],
        ret: &[Type::Void],
        target: None,
    },
    Prototype {
        builtin: Builtin::Print,
        namespace: None,
        name: "print",
        args: &[Type::String],
        ret: &[Type::Void],
        target: None,
    },
    Prototype {
        builtin: Builtin::Require,
        namespace: None,
        name: "require",
        args: &[Type::Bool],
        ret: &[Type::Void],
        target: None,
    },
    Prototype {
        builtin: Builtin::Require,
        namespace: None,
        name: "require",
        args: &[Type::Bool, Type::String],
        ret: &[Type::Void],
        target: None,
    },
    Prototype {
        builtin: Builtin::Revert,
        namespace: None,
        name: "revert",
        args: &[],
        ret: &[Type::Unreachable],
        target: None,
    },
    Prototype {
        builtin: Builtin::Revert,
        namespace: None,
        name: "revert",
        args: &[Type::String],
        ret: &[Type::Unreachable],
        target: None,
    },
    Prototype {
        builtin: Builtin::SelfDestruct,
        namespace: None,
        name: "selfdestruct",
        args: &[Type::Address(true)],
        ret: &[Type::Unreachable],
        target: None,
    },
    Prototype {
        builtin: Builtin::Keccak256,
        namespace: None,
        name: "keccak256",
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Ripemd160,
        namespace: None,
        name: "ripemd160",
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(20)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Sha256,
        namespace: None,
        name: "sha256",
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Blake2_128,
        namespace: None,
        name: "blake2_128",
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(16)],
        target: Some(Target::Substrate),
    },
    Prototype {
        builtin: Builtin::Blake2_256,
        namespace: None,
        name: "blake2_256",
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: Some(Target::Substrate),
    },
    Prototype {
        builtin: Builtin::Gasleft,
        namespace: None,
        name: "gasleft",
        args: &[],
        ret: &[Type::Uint(64)],
        target: None,
    },
    Prototype {
        builtin: Builtin::BlockHash,
        namespace: None,
        name: "blockhash",
        args: &[Type::Uint(64)],
        ret: &[Type::Bytes(32)],
        target: Some(Target::Ewasm),
    },
    Prototype {
        builtin: Builtin::Random,
        namespace: None,
        name: "random",
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: Some(Target::Substrate),
    },
    Prototype {
        builtin: Builtin::AbiDecode,
        namespace: Some("abi"),
        name: "decode",
        args: &[Type::DynamicBytes],
        ret: &[],
        target: None,
    },
    Prototype {
        builtin: Builtin::AbiEncode,
        namespace: Some("abi"),
        name: "encode",
        args: &[],
        ret: &[],
        target: None,
    },
    Prototype {
        builtin: Builtin::AbiEncodePacked,
        namespace: Some("abi"),
        name: "encodePacked",
        args: &[],
        ret: &[],
        target: None,
    },
    Prototype {
        builtin: Builtin::AbiEncodeWithSelector,
        namespace: Some("abi"),
        name: "encodeWithSelector",
        args: &[Type::Bytes(4)],
        ret: &[],
        target: None,
    },
    Prototype {
        builtin: Builtin::AbiEncodeWithSignature,
        namespace: Some("abi"),
        name: "encodeWithSignature",
        args: &[Type::String],
        ret: &[],
        target: None,
    },
    Prototype {
        builtin: Builtin::MulMod,
        namespace: None,
        name: "mulmod",
        args: &[Type::Uint(256), Type::Uint(256), Type::Uint(256)],
        ret: &[Type::Uint(256)],
        target: None,
    },
    Prototype {
        builtin: Builtin::AddMod,
        namespace: None,
        name: "addmod",
        args: &[Type::Uint(256), Type::Uint(256), Type::Uint(256)],
        ret: &[Type::Uint(256)],
        target: None,
    },
];

// A list of all Solidity builtins variables
static BUILTIN_VARIABLE: [Prototype; 14] = [
    Prototype {
        builtin: Builtin::BlockCoinbase,
        namespace: Some("block"),
        name: "coinbase",
        args: &[],
        ret: &[Type::Address(true)],
        target: Some(Target::Ewasm),
    },
    Prototype {
        builtin: Builtin::BlockDifficulty,
        namespace: Some("block"),
        name: "difficulty",
        args: &[],
        ret: &[Type::Uint(256)],
        target: Some(Target::Ewasm),
    },
    Prototype {
        builtin: Builtin::GasLimit,
        namespace: Some("block"),
        name: "gaslimit",
        args: &[],
        ret: &[Type::Uint(64)],
        target: Some(Target::Ewasm),
    },
    Prototype {
        builtin: Builtin::BlockNumber,
        namespace: Some("block"),
        name: "number",
        args: &[],
        ret: &[Type::Uint(64)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Timestamp,
        namespace: Some("block"),
        name: "timestamp",
        args: &[],
        ret: &[Type::Uint(64)],
        target: None,
    },
    Prototype {
        builtin: Builtin::TombstoneDeposit,
        namespace: Some("block"),
        name: "tombstone_deposit",
        args: &[],
        ret: &[Type::Uint(128)],
        target: Some(Target::Substrate),
    },
    Prototype {
        builtin: Builtin::MinimumBalance,
        namespace: Some("block"),
        name: "minimum_balance",
        args: &[],
        ret: &[Type::Uint(128)],
        target: Some(Target::Substrate),
    },
    Prototype {
        builtin: Builtin::Calldata,
        namespace: Some("msg"),
        name: "data",
        args: &[],
        ret: &[Type::DynamicBytes],
        target: None,
    },
    Prototype {
        builtin: Builtin::Sender,
        namespace: Some("msg"),
        name: "sender",
        args: &[],
        ret: &[Type::Address(true)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Signature,
        namespace: Some("msg"),
        name: "sig",
        args: &[],
        ret: &[Type::Bytes(4)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Value,
        namespace: Some("msg"),
        name: "value",
        args: &[],
        ret: &[Type::Uint(128)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Timestamp,
        namespace: None,
        name: "now",
        args: &[],
        ret: &[Type::Uint(64)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Gasprice,
        namespace: Some("tx"),
        name: "gasprice",
        args: &[],
        ret: &[Type::Uint(128)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Origin,
        namespace: Some("tx"),
        name: "origin",
        args: &[],
        ret: &[Type::Address(true)],
        target: Some(Target::Ewasm),
    },
];

/// Does function call match builtin
pub fn is_builtin_call(namespace: Option<&str>, fname: &str, ns: &Namespace) -> bool {
    BUILTIN_FUNCTIONS.iter().any(|p| {
        p.name == fname
            && p.namespace == namespace
            && (p.target.is_none() || p.target == Some(ns.target))
    })
}

/// Does variable name match builtin
pub fn builtin_var(
    namespace: Option<&str>,
    fname: &str,
    ns: &Namespace,
) -> Option<(Builtin, Type)> {
    if let Some(p) = BUILTIN_VARIABLE
        .iter()
        .find(|p| p.name == fname && p.namespace == namespace)
    {
        if p.target.is_none() || p.target == Some(ns.target) {
            return Some((p.builtin.clone(), p.ret[0].clone()));
        }
    }

    None
}

/// Is name reserved for builtins
pub fn is_reserved(fname: &str) -> bool {
    if fname == "type" {
        return true;
    }

    if BUILTIN_FUNCTIONS
        .iter()
        .any(|p| (p.name == fname && p.namespace == None) || (p.namespace == Some(fname)))
    {
        return true;
    }

    BUILTIN_VARIABLE
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
    let matches = BUILTIN_FUNCTIONS
        .iter()
        .filter(|p| p.name == id.name && p.namespace.is_none())
        .collect::<Vec<&Prototype>>();
    let marker = ns.diagnostics.len();
    for func in &matches {
        if func.args.len() != args.len() {
            ns.diagnostics.push(Diagnostic::error(
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
            match cast(&pt::Loc(0, 0, 0), arg.clone(), &func.args[i], true, ns) {
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
        ns.diagnostics.push(Diagnostic::error(
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
    file_no: usize,
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
            ns.diagnostics.push(Diagnostic::error(
                *loc,
                format!("function expects {} arguments, {} provided", 2, args.len()),
            ));

            return Err(());
        }

        // first args
        let data = cast(
            &args[0].loc(),
            expression(&args[0], file_no, contract_no, ns, symtable, false)?,
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
                        let ty = ns.resolve_type(file_no, contract_no, false, &param.ty)?;

                        if let Some(storage) = &param.storage {
                            ns.diagnostics.push(Diagnostic::error(
                                *storage.loc(),
                                format!("storage modifier ‘{}’ not allowed", storage),
                            ));
                            broken = true;
                        }

                        if let Some(name) = &param.name {
                            ns.diagnostics.push(Diagnostic::error(
                                name.loc,
                                format!("unexpected identifier ‘{}’ in type", name.name),
                            ));
                            broken = true;
                        }

                        if ty.is_mapping() {
                            ns.diagnostics.push(Diagnostic::error(
                                *loc,
                                "mapping cannot be abi decoded or encoded".to_string(),
                            ));
                            broken = true;
                        }

                        tys.push(ty);
                    } else {
                        ns.diagnostics
                            .push(Diagnostic::error(*loc, "missing type".to_string()));

                        broken = true;
                    }
                }
            }
            _ => {
                let ty = ns.resolve_type(file_no, contract_no, false, &args[1])?;

                if ty.is_mapping() {
                    ns.diagnostics.push(Diagnostic::error(
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
                let selector = expression(selector, file_no, contract_no, ns, symtable, false)?;

                resolved_args.insert(
                    0,
                    cast(&selector.loc(), selector, &Type::Bytes(4), true, ns)?,
                );
            } else {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "function requires one ‘bytes4’ selector argument".to_string(),
                ));

                return Err(());
            }
        }
        Builtin::AbiEncodeWithSignature => {
            // first argument is signature
            if let Some(signature) = args_iter.next() {
                let signature = expression(signature, file_no, contract_no, ns, symtable, false)?;

                resolved_args.insert(
                    0,
                    cast(&signature.loc(), signature, &Type::String, true, ns)?,
                );
            } else {
                ns.diagnostics.push(Diagnostic::error(
                    *loc,
                    "function requires one ‘string’ signature argument".to_string(),
                ));

                return Err(());
            }
        }
        _ => (),
    }

    for arg in args_iter {
        let mut expr = expression(arg, file_no, contract_no, ns, symtable, false)?;
        let ty = expr.ty();

        if ty.is_mapping() {
            ns.diagnostics.push(Diagnostic::error(
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
