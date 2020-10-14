extern crate blake2_rfc;
extern crate clap;
extern crate contract_metadata;
extern crate hex;
extern crate inkwell;
extern crate lalrpop_util;
extern crate lazy_static;
extern crate num_bigint;
extern crate num_derive;
extern crate num_traits;
extern crate parity_wasm;
extern crate phf;
extern crate semver;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate tiny_keccak;
extern crate unicode_xid;

pub mod abi;
pub mod codegen;
mod emit;
pub mod file_cache;
pub mod link;
pub mod parser;
pub mod sema;

use file_cache::FileCache;
use inkwell::OptimizationLevel;
use sema::ast;
use sema::diagnostics;
use std::fmt;

/// The target chain you want to compile Solidity for.
#[derive(PartialEq, Clone, Copy)]
pub enum Target {
    /// Parity Substrate, see https://substrate.dev/
    Substrate,
    /// Ethereum ewasm, see https://github.com/ewasm/design
    Ewasm,
    /// Sawtooth Sabre, see https://github.com/hyperledger/sawtooth-sabre
    Sabre,
    /// Generate a generic object file for linking
    Generic,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Target::Substrate => write!(f, "Substrate"),
            Target::Ewasm => write!(f, "ewasm"),
            Target::Sabre => write!(f, "Sawtooth Sabre"),
            Target::Generic => write!(f, "generic"),
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
) -> (Vec<(Vec<u8>, String)>, ast::Namespace) {
    let ctx = inkwell::context::Context::create();

    let mut ns = parse_and_resolve(filename, cache, target);

    if diagnostics::any_errors(&ns.diagnostics) {
        return (Vec::new(), ns);
    }

    // codegen all the contracts
    for contract_no in 0..ns.contracts.len() {
        codegen::codegen(contract_no, &mut ns);
    }

    let results = (0..ns.contracts.len())
        .filter(|c| ns.contracts[*c].is_concrete())
        .map(|c| {
            // codegen
            let contract = emit::Contract::build(&ctx, &ns.contracts[c], &ns, filename, opt);

            let bc = contract.wasm(true).expect("llvm wasm emit should work");

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
        },
        16,
    );

    if let Err(message) = cache.populate_cache(filename) {
        ns.diagnostics.push(ast::Diagnostic {
            ty: ast::ErrorType::ParserError,
            level: ast::Level::Error,
            message,
            pos: None,
            notes: Vec::new(),
        });
    } else {
        // resolve
        sema::sema(filename, cache, &mut ns);
    }

    ns
}
