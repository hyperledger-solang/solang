pub mod abi;
pub mod codegen;
mod emit;
pub mod file_cache;
pub mod linker;
pub mod parser;

// In Sema, we use result unit for returning early
// when code-misparses. The error will be added to the namespace diagnostics, no need to have anything but unit
// as error.
#[allow(clippy::result_unit_err)]
pub mod sema;

use file_cache::FileCache;
use inkwell::OptimizationLevel;
use sema::ast;
use sema::diagnostics;
use std::fmt;

/// The target chain you want to compile Solidity for.
#[derive(PartialEq, Clone, Copy)]
pub enum Target {
    /// Parity Substrate, see <https://substrate.dev/>
    Substrate,
    /// Ethereum ewasm, see <https://github.com/ewasm/design>
    Ewasm,
    /// Sawtooth Sabre, see <https://github.com/hyperledger/sawtooth-sabre>
    Sabre,
    /// Generate a generic object file for linking
    Generic,
    /// Solana, see <https://solana.com/>
    Solana,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Target::Substrate => write!(f, "Substrate"),
            Target::Ewasm => write!(f, "ewasm"),
            Target::Sabre => write!(f, "Sawtooth Sabre"),
            Target::Generic => write!(f, "generic"),
            Target::Solana => write!(f, "solana"),
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
pub fn compile(
    filename: &str,
    cache: &mut FileCache,
    opt: OptimizationLevel,
    target: Target,
    math_overflow_check: bool,
) -> (Vec<(Vec<u8>, String)>, ast::Namespace) {
    let ctx = inkwell::context::Context::create();

    let mut ns = parse_and_resolve(filename, cache, target);

    if diagnostics::any_errors(&ns.diagnostics) {
        return (Vec::new(), ns);
    }

    // codegen all the contracts
    for contract_no in 0..ns.contracts.len() {
        codegen::codegen(contract_no, &mut ns, &Default::default());
    }

    let results = (0..ns.contracts.len())
        .filter(|c| ns.contracts[*c].is_concrete())
        .map(|c| {
            // codegen
            let contract = emit::Contract::build(
                &ctx,
                &ns.contracts[c],
                &ns,
                filename,
                opt,
                math_overflow_check,
            );

            let bc = contract.code(true).expect("llvm code emit should work");

            let (abistr, _) = abi::generate_abi(c, &ns, &bc, false);

            (bc, abistr)
        })
        .collect();

    (results, ns)
}

/// Parse and resolve the Solidity source code provided in src, for the target chain as specified in target.
/// The result is a list of resolved contracts (if successful) and a list of compiler warnings, errors and
/// informational messages like `found contact N`.
///
/// Note that multiple contracts can be specified in on solidity source file.
pub fn parse_and_resolve(filename: &str, cache: &mut FileCache, target: Target) -> ast::Namespace {
    let mut ns = ast::Namespace::new(
        target,
        match target {
            Target::Ewasm => 20,
            Target::Substrate => 32,
            Target::Sabre => 0,    // Sabre has no address type
            Target::Generic => 20, // Same as ethereum
            Target::Solana => 32,
        },
        if target == Target::Solana {
            8 // lamports is u64
        } else {
            16 // value is 128 bits
        },
    );

    match cache.resolve_file(None, filename) {
        Err(message) => {
            ns.diagnostics.push(ast::Diagnostic {
                ty: ast::ErrorType::ParserError,
                level: ast::Level::Error,
                message,
                pos: None,
                notes: Vec::new(),
            });
        }
        Ok(file) => {
            sema::sema(file, cache, &mut ns);
        }
    }

    ns
}
