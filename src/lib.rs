// SPDX-License-Identifier: Apache-2.0

pub mod abi;
pub mod codegen;
#[cfg(feature = "llvm")]
pub mod emit;
pub mod file_resolver;
#[cfg(feature = "llvm")]
mod linker;
pub mod standard_json;

// In Sema, we use result unit for returning early
// when code-misparses. The error will be added to the namespace diagnostics, no need to have anything but unit
// as error.
pub mod sema;

use file_resolver::FileResolver;
use sema::diagnostics;
use solang_parser::pt;
use std::{ffi::OsStr, fmt};

/// The target chain you want to compile Solidity for.
#[derive(Clone, Copy)]
pub enum Target {
    /// Solana, see <https://solana.com/>
    Solana,
    /// Parity Substrate, see <https://substrate.io/>
    Substrate {
        address_length: usize,
        value_length: usize,
    },
    /// Ethereum EVM, see <https://ethereum.org/en/developers/docs/evm/>
    EVM,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Target::Solana => write!(f, "solana"),
            Target::Substrate { .. } => write!(f, "substrate"),
            Target::EVM => write!(f, "evm"),
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
            Target::EVM => matches!(other, Target::EVM),
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
            "evm" => Some(Target::EVM),
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

    /// This function returns the byte length for a selector, given the target
    pub fn selector_length(&self) -> u8 {
        match self {
            Target::Solana => 8,
            _ => 4,
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
    log_api_return_codes: bool,
) -> (Vec<(Vec<u8>, String)>, sema::ast::Namespace) {
    let mut ns = parse_and_resolve(filename, resolver, target);
    let opts = codegen::Options {
        math_overflow_check,
        log_api_return_codes,
        opt_level: opt_level.into(),
        ..Default::default()
    };

    if ns.diagnostics.any_errors() {
        return (Vec::new(), ns);
    }

    // codegen all the contracts
    codegen::codegen(&mut ns, &opts);

    if ns.diagnostics.any_errors() {
        return (Vec::new(), ns);
    }

    // emit the contracts
    let mut results = Vec::new();

    for contract_no in 0..ns.contracts.len() {
        let contract = &ns.contracts[contract_no];

        if contract.instantiable {
            let code = contract.emit(&ns, &opts);

            let (abistr, _) = abi::generate_abi(contract_no, &ns, &code, false);

            results.push((code, abistr));
        };
    }

    (results, ns)
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
) -> sema::ast::Namespace {
    let mut ns = sema::ast::Namespace::new(target);

    match resolver.resolve_file(None, filename) {
        Err(message) => {
            ns.diagnostics.push(sema::ast::Diagnostic {
                ty: sema::ast::ErrorType::ParserError,
                level: sema::ast::Level::Error,
                message,
                loc: pt::Loc::CommandLine,
                notes: Vec::new(),
            });
        }
        Ok(file) => {
            sema::sema(&file, resolver, &mut ns);
        }
    }

    ns.diagnostics.sort_and_dedup();

    ns
}
