use super::ast::{Builtin, Diagnostic, Expression, Namespace, Type};
use super::eval::eval_const_number;
use super::expression::{cast, expression, ExprContext, ResolveTo};
use super::symtable::Symtable;
use crate::parser::pt;
use crate::Target;
use num_bigint::BigInt;
use num_traits::One;

pub struct Prototype {
    pub builtin: Builtin,
    pub namespace: Option<&'static str>,
    pub name: &'static str,
    pub args: &'static [Type],
    pub ret: &'static [Type],
    pub target: &'static [Target],
    pub doc: &'static str,
    // Can this function be called in constant context (e.g. hash functions)
    pub constant: bool,
}

// A list of all Solidity builtins functions
static BUILTIN_FUNCTIONS: [Prototype; 24] = [
    Prototype {
        builtin: Builtin::Assert,
        namespace: None,
        name: "assert",
        args: &[Type::Bool],
        ret: &[Type::Void],
        target: &[],
        doc: "Abort execution if argument evaluates to false",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Print,
        namespace: None,
        name: "print",
        args: &[Type::String],
        ret: &[Type::Void],
        target: &[],
        doc: "log string for debugging purposes. Runs on development chain only",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Require,
        namespace: None,
        name: "require",
        args: &[Type::Bool],
        ret: &[Type::Void],
        target: &[],
        doc: "Abort execution if argument evaulates to false",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Require,
        namespace: None,
        name: "require",
        args: &[Type::Bool, Type::String],
        ret: &[Type::Void],
        target: &[],
        doc: "Abort execution if argument evaulates to false. Report string when aborting",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Revert,
        namespace: None,
        name: "revert",
        args: &[],
        ret: &[Type::Unreachable],
        target: &[],
        doc: "Revert execution",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Revert,
        namespace: None,
        name: "revert",
        args: &[Type::String],
        ret: &[Type::Unreachable],
        target: &[],
        doc: "Revert execution and report string",
        constant: false,
    },
    Prototype {
        builtin: Builtin::SelfDestruct,
        namespace: None,
        name: "selfdestruct",
        args: &[Type::Address(true)],
        ret: &[Type::Unreachable],
        target: &[Target::Ewasm, Target::default_substrate()],
        doc: "Destroys current account and deposits any remaining balance to address",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Keccak256,
        namespace: None,
        name: "keccak256",
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: &[],
        doc: "Calculates keccak256 hash",
        constant: true,
    },
    Prototype {
        builtin: Builtin::Ripemd160,
        namespace: None,
        name: "ripemd160",
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(20)],
        target: &[],
        doc: "Calculates ripemd hash",
        constant: true,
    },
    Prototype {
        builtin: Builtin::Sha256,
        namespace: None,
        name: "sha256",
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: &[],
        doc: "Calculates sha256 hash",
        constant: true,
    },
    Prototype {
        builtin: Builtin::Blake2_128,
        namespace: None,
        name: "blake2_128",
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(16)],
        target: &[Target::default_substrate()],
        doc: "Calculates blake2-128 hash",
        constant: true,
    },
    Prototype {
        builtin: Builtin::Blake2_256,
        namespace: None,
        name: "blake2_256",
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: &[Target::default_substrate()],
        doc: "Calculates blake2-256 hash",
        constant: true,
    },
    Prototype {
        builtin: Builtin::Gasleft,
        namespace: None,
        name: "gasleft",
        args: &[],
        ret: &[Type::Uint(64)],
        target: &[Target::default_substrate(), Target::Ewasm],
        doc: "Return remaing gas left in current call",
        constant: false,
    },
    Prototype {
        builtin: Builtin::BlockHash,
        namespace: None,
        name: "blockhash",
        args: &[Type::Uint(64)],
        ret: &[Type::Bytes(32)],
        target: &[Target::Ewasm],
        doc: "Returns the block hash for given block number",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Random,
        namespace: None,
        name: "random",
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: &[Target::default_substrate()],
        doc: "Returns deterministic random bytes",
        constant: false,
    },
    Prototype {
        builtin: Builtin::AbiDecode,
        namespace: Some("abi"),
        name: "decode",
        args: &[Type::DynamicBytes],
        ret: &[],
        target: &[],
        doc: "Abi decode byte array with the given types",
        constant: false,
    },
    Prototype {
        builtin: Builtin::AbiEncode,
        namespace: Some("abi"),
        name: "encode",
        args: &[],
        ret: &[],
        target: &[],
        doc: "Abi encode given arguments",
        // it should be allowed in constant context, but we don't supported that yet
        constant: false,
    },
    Prototype {
        builtin: Builtin::AbiEncodePacked,
        namespace: Some("abi"),
        name: "encodePacked",
        args: &[],
        ret: &[],
        target: &[],
        doc: "Abi encode given arguments using packed encoding",
        // it should be allowed in constant context, but we don't supported that yet
        constant: false,
    },
    Prototype {
        builtin: Builtin::AbiEncodeWithSelector,
        namespace: Some("abi"),
        name: "encodeWithSelector",
        args: &[Type::Bytes(4)],
        ret: &[],
        target: &[],
        doc: "Abi encode given arguments with selector",
        // it should be allowed in constant context, but we don't supported that yet
        constant: false,
    },
    Prototype {
        builtin: Builtin::AbiEncodeWithSignature,
        namespace: Some("abi"),
        name: "encodeWithSignature",
        args: &[Type::String],
        ret: &[],
        target: &[],
        doc: "Abi encode given arguments with function signature",
        // it should be allowed in constant context, but we don't supported that yet
        constant: false,
    },
    Prototype {
        builtin: Builtin::Gasprice,
        namespace: Some("tx"),
        name: "gasprice",
        args: &[Type::Uint(64)],
        ret: &[Type::Value],
        target: &[],
        doc: "Calculate price of given gas units",
        constant: false,
    },
    Prototype {
        builtin: Builtin::MulMod,
        namespace: None,
        name: "mulmod",
        args: &[Type::Uint(256), Type::Uint(256), Type::Uint(256)],
        ret: &[Type::Uint(256)],
        target: &[],
        doc: "Multiply first two arguments, and the modulo last argument. Does not overflow",
        // it should be allowed in constant context, but we don't supported that yet
        constant: false,
    },
    Prototype {
        builtin: Builtin::AddMod,
        namespace: None,
        name: "addmod",
        args: &[Type::Uint(256), Type::Uint(256), Type::Uint(256)],
        ret: &[Type::Uint(256)],
        target: &[],
        doc: "Add first two arguments, and the modulo last argument. Does not overflow",
        // it should be allowed in constant context, but we don't supported that yet
        constant: false,
    },
    Prototype {
        builtin: Builtin::SignatureVerify,
        namespace: None,
        name: "signatureVerify",
        args: &[Type::Address(false), Type::DynamicBytes, Type::DynamicBytes],
        ret: &[Type::Bool],
        target: &[Target::Solana],
        doc: "ed25519 signature verification",
        constant: false,
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
        target: &[Target::Ewasm],
        doc: "The address of the current block miner",
        constant: false,
    },
    Prototype {
        builtin: Builtin::BlockDifficulty,
        namespace: Some("block"),
        name: "difficulty",
        args: &[],
        ret: &[Type::Uint(256)],
        target: &[Target::Ewasm],
        doc: "The difficulty for current block",
        constant: false,
    },
    Prototype {
        builtin: Builtin::GasLimit,
        namespace: Some("block"),
        name: "gaslimit",
        args: &[],
        ret: &[Type::Uint(64)],
        target: &[Target::Ewasm],
        doc: "The gas limit",
        constant: false,
    },
    Prototype {
        builtin: Builtin::BlockNumber,
        namespace: Some("block"),
        name: "number",
        args: &[],
        ret: &[Type::Uint(64)],
        target: &[],
        doc: "Current block number",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Slot,
        namespace: Some("block"),
        name: "slot",
        args: &[],
        ret: &[Type::Uint(64)],
        target: &[Target::Solana],
        doc: "Current slot number",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Timestamp,
        namespace: Some("block"),
        name: "timestamp",
        args: &[],
        ret: &[Type::Uint(64)],
        target: &[],
        doc: "Current timestamp in unix epoch (seconds since 1970)",
        constant: false,
    },
    Prototype {
        builtin: Builtin::TombstoneDeposit,
        namespace: Some("block"),
        name: "tombstone_deposit",
        args: &[],
        ret: &[Type::Value],
        target: &[Target::default_substrate()],
        doc: "Deposit required for a tombstone",
        constant: false,
    },
    Prototype {
        builtin: Builtin::MinimumBalance,
        namespace: Some("block"),
        name: "minimum_balance",
        args: &[],
        ret: &[Type::Value],
        target: &[Target::default_substrate()],
        doc: "Minimum balance required for an account",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Calldata,
        namespace: Some("msg"),
        name: "data",
        args: &[],
        ret: &[Type::DynamicBytes],
        target: &[],
        doc: "Raw input bytes to current call",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Sender,
        namespace: Some("msg"),
        name: "sender",
        args: &[],
        ret: &[Type::Address(true)],
        target: &[],
        constant: false,
        doc: "Address of caller",
    },
    Prototype {
        builtin: Builtin::Signature,
        namespace: Some("msg"),
        name: "sig",
        args: &[],
        ret: &[Type::Bytes(4)],
        target: &[],
        doc: "Function selector for current call",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Value,
        namespace: Some("msg"),
        name: "value",
        args: &[],
        ret: &[Type::Value],
        target: &[],
        doc: "Value sent with current call",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Gasprice,
        namespace: Some("tx"),
        name: "gasprice",
        args: &[],
        ret: &[Type::Value],
        target: &[Target::default_substrate(), Target::Ewasm],
        doc: "gas price for one gas unit",
        constant: false,
    },
    Prototype {
        builtin: Builtin::Origin,
        namespace: Some("tx"),
        name: "origin",
        args: &[],
        ret: &[Type::Address(true)],
        target: &[Target::Ewasm],
        doc: "Original address of sender current transaction",
        constant: false,
    },
];

/// Does function call match builtin
pub fn is_builtin_call(namespace: Option<&str>, fname: &str, ns: &Namespace) -> bool {
    BUILTIN_FUNCTIONS.iter().any(|p| {
        p.name == fname
            && p.namespace == namespace
            && (p.target.is_empty() || p.target.contains(&ns.target))
    })
}

/// Get the prototype for a builtin. If the prototype has arguments, it is a function else
/// it is a variable.
pub fn get_prototype(builtin: Builtin) -> Option<&'static Prototype> {
    BUILTIN_FUNCTIONS
        .iter()
        .find(|p| p.builtin == builtin)
        .or_else(|| BUILTIN_VARIABLE.iter().find(|p| p.builtin == builtin))
}

/// Does variable name match builtin
pub fn builtin_var(
    loc: &pt::Loc,
    namespace: Option<&str>,
    fname: &str,
    ns: &Namespace,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(Builtin, Type)> {
    if let Some(p) = BUILTIN_VARIABLE
        .iter()
        .find(|p| p.name == fname && p.namespace == namespace)
    {
        if p.target.is_empty() || p.target.contains(&ns.target) {
            if ns.target.is_substrate() && p.builtin == Builtin::Gasprice {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    String::from(
                        "use the function ‘tx.gasprice(gas)’ in stead, as ‘tx.gasprice’ may round down to zero. See https://solang.readthedocs.io/en/latest/language.html#gasprice",
                    ),
                ));
            }
            return Some((p.builtin, p.ret[0].clone()));
        }
    }

    None
}

/// Does variable name match any builtin namespace
pub fn builtin_namespace(namespace: &str) -> bool {
    BUILTIN_VARIABLE
        .iter()
        .any(|p| p.namespace == Some(namespace))
}

/// Is name reserved for builtins
pub fn is_reserved(fname: &str) -> bool {
    if fname == "type" || fname == "super" {
        return true;
    }

    let is_builtin_function = BUILTIN_FUNCTIONS
        .iter()
        .any(|p| (p.name == fname && p.namespace == None) || (p.namespace == Some(fname)));

    if is_builtin_function {
        return true;
    }

    BUILTIN_VARIABLE
        .iter()
        .any(|p| (p.name == fname && p.namespace == None) || (p.namespace == Some(fname)))
}

/// Resolve a builtin call
pub fn resolve_call(
    loc: &pt::Loc,
    namespace: Option<&str>,
    id: &str,
    args: &[pt::Expression],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    let matches = BUILTIN_FUNCTIONS
        .iter()
        .filter(|p| p.name == id && p.namespace == namespace)
        .collect::<Vec<&Prototype>>();

    let marker = diagnostics.len();

    for func in &matches {
        if context.constant && !func.constant {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!(
                    "cannot call function ‘{}’ in constant expression",
                    func.name
                ),
            ));
            return Err(());
        }

        if func.args.len() != args.len() {
            diagnostics.push(Diagnostic::error(
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
            let arg = match expression(
                arg,
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&func.args[i]),
            ) {
                Ok(e) => e,
                Err(()) => {
                    matches = false;
                    continue;
                }
            };

            match cast(
                &arg.loc(),
                arg.clone(),
                &func.args[i],
                true,
                ns,
                diagnostics,
            ) {
                Ok(expr) => cast_args.push(expr),
                Err(()) => {
                    matches = false;
                    continue;
                }
            }
        }

        if matches {
            diagnostics.truncate(marker);

            // tx.gasprice(1) is a bad idea, just like tx.gasprice. Warn about this
            if ns.target.is_substrate() && func.builtin == Builtin::Gasprice {
                if let Ok((_, val)) = eval_const_number(&cast_args[0], context.contract_no, ns) {
                    if val == BigInt::one() {
                        diagnostics.push(Diagnostic::warning(
                            *loc,
                            String::from(
                                "the function call ‘tx.gasprice(1)’ may round down to zero. See https://solang.readthedocs.io/en/latest/language.html#gasprice",
                            ),
                        ));
                    }
                }
            }

            return Ok(Expression::Builtin(
                *loc,
                func.ret.to_vec(),
                func.builtin,
                cast_args,
            ));
        }
    }

    if matches.len() != 1 {
        diagnostics.truncate(marker);
        diagnostics.push(Diagnostic::error(
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
pub fn resolve_namespace_call(
    loc: &pt::Loc,
    namespace: &str,
    name: &str,
    args: &[pt::Expression],
    context: &ExprContext,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    // The abi.* functions need special handling, others do not
    if namespace != "abi" {
        return resolve_call(
            loc,
            Some(namespace),
            name,
            args,
            context,
            ns,
            symtable,
            diagnostics,
        );
    }

    let builtin = match name {
        "decode" => Builtin::AbiDecode,
        "encode" => Builtin::AbiEncode,
        "encodePacked" => Builtin::AbiEncodePacked,
        "encodeWithSelector" => Builtin::AbiEncodeWithSelector,
        "encodeWithSignature" => Builtin::AbiEncodeWithSignature,
        _ => unreachable!(),
    };

    if builtin == Builtin::AbiDecode {
        if args.len() != 2 {
            diagnostics.push(Diagnostic::error(
                *loc,
                format!("function expects {} arguments, {} provided", 2, args.len()),
            ));

            return Err(());
        }

        // first args
        let data = cast(
            &args[0].loc(),
            expression(
                &args[0],
                context,
                ns,
                symtable,
                diagnostics,
                ResolveTo::Type(&Type::DynamicBytes),
            )?,
            &Type::DynamicBytes,
            true,
            ns,
            diagnostics,
        )?;

        let mut tys = Vec::new();
        let mut broken = false;

        match &args[1] {
            pt::Expression::List(_, list) => {
                for (loc, param) in list {
                    if let Some(param) = param {
                        let ty = ns.resolve_type(
                            context.file_no,
                            context.contract_no,
                            false,
                            &param.ty,
                            diagnostics,
                        )?;

                        if let Some(storage) = &param.storage {
                            diagnostics.push(Diagnostic::error(
                                *storage.loc(),
                                format!("storage modifier ‘{}’ not allowed", storage),
                            ));
                            broken = true;
                        }

                        if let Some(name) = &param.name {
                            diagnostics.push(Diagnostic::error(
                                name.loc,
                                format!("unexpected identifier ‘{}’ in type", name.name),
                            ));
                            broken = true;
                        }

                        if ty.is_mapping() {
                            diagnostics.push(Diagnostic::error(
                                *loc,
                                "mapping cannot be abi decoded or encoded".to_string(),
                            ));
                            broken = true;
                        }

                        tys.push(ty);
                    } else {
                        diagnostics.push(Diagnostic::error(*loc, "missing type".to_string()));

                        broken = true;
                    }
                }
            }
            _ => {
                let ty = ns.resolve_type(
                    context.file_no,
                    context.contract_no,
                    false,
                    &args[1],
                    diagnostics,
                )?;

                if ty.is_mapping() {
                    diagnostics.push(Diagnostic::error(
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
                let selector = expression(
                    selector,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&Type::Bytes(4)),
                )?;

                resolved_args.insert(
                    0,
                    cast(
                        &selector.loc(),
                        selector,
                        &Type::Bytes(4),
                        true,
                        ns,
                        diagnostics,
                    )?,
                );
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "function requires one ‘bytes4’ selector argument".to_string(),
                ));

                return Err(());
            }
        }
        Builtin::AbiEncodeWithSignature => {
            // first argument is signature
            if let Some(signature) = args_iter.next() {
                let signature = expression(
                    signature,
                    context,
                    ns,
                    symtable,
                    diagnostics,
                    ResolveTo::Type(&Type::String),
                )?;

                resolved_args.insert(
                    0,
                    cast(
                        &signature.loc(),
                        signature,
                        &Type::String,
                        true,
                        ns,
                        diagnostics,
                    )?,
                );
            } else {
                diagnostics.push(Diagnostic::error(
                    *loc,
                    "function requires one ‘string’ signature argument".to_string(),
                ));

                return Err(());
            }
        }
        _ => (),
    }

    for arg in args_iter {
        let mut expr = expression(arg, context, ns, symtable, diagnostics, ResolveTo::Unknown)?;
        let ty = expr.ty();

        if ty.is_mapping() {
            diagnostics.push(Diagnostic::error(
                arg.loc(),
                "mapping type not permitted".to_string(),
            ));

            return Err(());
        }

        expr = cast(&arg.loc(), expr, ty.deref_any(), true, ns, diagnostics)?;

        // A string or hex literal should be encoded as a string
        if let Expression::BytesLiteral(_, _, _) = &expr {
            expr = cast(&arg.loc(), expr, &Type::String, true, ns, diagnostics)?;
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
