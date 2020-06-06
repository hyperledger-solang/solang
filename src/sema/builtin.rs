use crate::Target;
use output::Output;
use parser::pt;
use sema::ast::{Builtin, Expression, Namespace, Type};
use sema::expression::cast;

struct Prototype {
    pub builtin: Builtin,
    pub this: Option<Type>,
    pub name: &'static str,
    pub args: &'static [Type],
    pub ret: &'static [Type],
    pub target: Option<Target>,
}

// A list of all Solidity builtins
static PROTO_TYPES: [Prototype; 12] = [
    Prototype {
        builtin: Builtin::Assert,
        name: "assert",
        this: None,
        args: &[Type::Bool],
        ret: &[Type::Void],
        target: None,
    },
    Prototype {
        builtin: Builtin::Print,
        name: "print",
        this: None,
        args: &[Type::String],
        ret: &[Type::Void],
        target: None,
    },
    Prototype {
        builtin: Builtin::Require,
        name: "require",
        this: None,
        args: &[Type::Bool],
        ret: &[Type::Void],
        target: None,
    },
    Prototype {
        builtin: Builtin::Require,
        name: "require",
        this: None,
        args: &[Type::Bool, Type::String],
        ret: &[Type::Void],
        target: None,
    },
    Prototype {
        builtin: Builtin::Revert,
        name: "revert",
        this: None,
        args: &[],
        ret: &[Type::Unreachable],
        target: None,
    },
    Prototype {
        builtin: Builtin::Revert,
        name: "revert",
        this: None,
        args: &[Type::String],
        ret: &[Type::Unreachable],
        target: None,
    },
    Prototype {
        builtin: Builtin::SelfDestruct,
        name: "selfdestruct",
        this: None,
        args: &[Type::Address(true)],
        ret: &[Type::Unreachable],
        target: None,
    },
    Prototype {
        builtin: Builtin::Keccak256,
        name: "keccak256",
        this: None,
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Ripemd160,
        name: "ripemd160",
        this: None,
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(20)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Sha256,
        name: "sha256",
        this: None,
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: None,
    },
    Prototype {
        builtin: Builtin::Blake2_128,
        name: "blake2_128",
        this: None,
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(16)],
        target: Some(Target::Substrate),
    },
    Prototype {
        builtin: Builtin::Blake2_256,
        name: "blake2_256",
        this: None,
        args: &[Type::DynamicBytes],
        ret: &[Type::Bytes(32)],
        target: Some(Target::Substrate),
    },
];

/// Does function call match builtin
pub fn is_builtin_call(fname: &str) -> bool {
    PROTO_TYPES
        .iter()
        .any(|p| p.name == fname && p.this.is_none())
}

/// Resolve a builtin call
pub fn resolve(
    loc: &pt::Loc,
    id: &pt::Identifier,
    args: Vec<Expression>,
    ns: &mut Namespace,
) -> Result<Expression, ()> {
    let matches = PROTO_TYPES
        .iter()
        .filter(|p| p.name == id.name && p.this.is_none())
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
