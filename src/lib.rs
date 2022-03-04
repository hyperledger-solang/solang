pub mod abi;
pub mod codegen;
#[cfg(feature = "llvm")]
pub mod emit;
pub mod file_resolver;
#[cfg(feature = "llvm")]
pub mod linker;
pub use solang_parser as parser;
// In Sema, we use result unit for returning early
// when code-misparses. The error will be added to the namespace diagnostics, no need to have anything but unit
// as error.
#[allow(clippy::result_unit_err)]
pub mod sema;

use file_resolver::FileResolver;
use sema::ast;
use sema::diagnostics;
use solang_parser::pt;
use std::{ffi::OsStr, fmt};

/// The target chain you want to compile Solidity for.
#[derive(Clone, Copy)]
pub enum Target {
    /// Solana, see <https://solana.com/>
    Solana,
    /// Parity Substrate, see <https://substrate.dev/>
    Substrate {
        address_length: usize,
        value_length: usize,
    },
    /// Ethereum ewasm, see <https://github.com/ewasm/design>
    Ewasm,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Target::Solana => write!(f, "solana"),
            Target::Substrate { .. } => write!(f, "substrate"),
            Target::Ewasm => write!(f, "ewasm"),
        }
    }
}

impl PartialEq for Target {
    // Equality should check if it the same chain, not compare parameters. This
    // is needed for builtins for example
    fn eq(&self, other: &Self) -> bool {
        match self {
            Target::Solana => matches!(other, Target::Solana),
            Target::Substrate { .. } => matches!(other, Target::Substrate { .. }),
            Target::Ewasm => matches!(other, Target::Ewasm),
        }
    }
}

impl Target {
    /// Short-hand for checking for Substrate target
    pub fn is_substrate(&self) -> bool {
        matches!(self, Target::Substrate { .. })
    }

    /// Create the target Substrate with default parameters
    pub const fn default_substrate() -> Self {
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        }
    }

    /// Creates a target from a string
    pub fn from(name: &str) -> Option<Self> {
        match name {
            "solana" => Some(Target::Solana),
            "substrate" => Some(Target::default_substrate()),
            "ewasm" => Some(Target::Ewasm),
            _ => None,
        }
    }

    /// File extension
    pub fn file_extension(&self) -> &'static str {
        match self {
            // Solana uses ELF dynamic shared object (BPF)
            Target::Solana => "so",
            // Everything else generates webassembly
            _ => "wasm",
        }
    }

    /// Size of a pointer in bits
    pub fn ptr_size(&self) -> u16 {
        if *self == Target::Solana {
            // Solana is BPF, which is 64 bit
            64
        } else {
            // All others are WebAssembly in 32 bit mode
            32
        }
    }
}

/// Compile a solidity file to list of wasm files and their ABIs. The filename is only used for error messages;
/// the contents of the file is provided in the `src` argument.
///
/// This function only produces a single contract and abi, which is compiled for the `target` specified. Any
/// compiler warnings, errors and informational messages are also provided.
///
/// The ctx is the inkwell llvm context.
#[cfg(feature = "llvm")]
pub fn compile(
    filename: &OsStr,
    resolver: &mut FileResolver,
    opt_level: inkwell::OptimizationLevel,
    target: Target,
    math_overflow_check: bool,
) -> (Vec<(Vec<u8>, String)>, ast::Namespace) {
    let mut ns = parse_and_resolve(filename, resolver, target);

    if diagnostics::any_errors(&ns.diagnostics) {
        return (Vec::new(), ns);
    }

    // codegen all the contracts
    codegen::codegen(
        &mut ns,
        &codegen::Options {
            math_overflow_check,
            opt_level: opt_level.into(),
            ..Default::default()
        },
    );

    let results = (0..ns.contracts.len())
        .filter(|c| ns.contracts[*c].is_concrete())
        .map(|c| {
            // codegen has already happened
            assert!(!ns.contracts[c].code.is_empty());

            let code = &ns.contracts[c].code;
            let (abistr, _) = abi::generate_abi(c, &ns, code, false);

            (code.clone(), abistr)
        })
        .collect();

    (results, ns)
}

/// Build a single binary out of multiple contracts. This is only possible on Solana
#[cfg(feature = "llvm")]
pub fn compile_many<'a>(
    context: &'a inkwell::context::Context,
    namespaces: &'a [ast::Namespace],
    filename: &str,
    opt: inkwell::OptimizationLevel,
    math_overflow_check: bool,
) -> emit::Binary<'a> {
    emit::Binary::build_bundle(context, namespaces, filename, opt, math_overflow_check)
}

/// Parse and resolve the Solidity source code provided in src, for the target chain as specified in target.
/// The result is a list of resolved contracts (if successful) and a list of compiler warnings, errors and
/// informational messages like `found contact N`.
///
/// Note that multiple contracts can be specified in on solidity source file.
pub fn parse_and_resolve(
    filename: &OsStr,
    resolver: &mut FileResolver,
    target: Target,
) -> ast::Namespace {
    let mut ns = ast::Namespace::new(target);

    match resolver.resolve_file(None, filename) {
        Err(message) => {
            ns.diagnostics.push(ast::Diagnostic {
                ty: ast::ErrorType::ParserError,
                level: ast::Level::Error,
                message,
                pos: pt::Loc::CommandLine,
                notes: Vec::new(),
            });
        }
        Ok(file) => {
            sema::sema(&file, resolver, &mut ns);
        }
    }

    ns
}
