extern crate clap;
extern crate hex;
extern crate lalrpop_util;
extern crate lazy_static;
extern crate num_bigint;
extern crate num_traits;
extern crate parity_wasm;
extern crate serde;
extern crate tiny_keccak;
extern crate unescape;
extern crate inkwell;
extern crate num_derive;
extern crate serde_derive;

pub mod link;
pub mod output;
pub mod abi;

mod emit;
mod parser;
mod resolver;

use std::fmt;

/// The target chain you want to compile Solidity for.
#[derive(PartialEq, Clone)]
pub enum Target {
    /// Parity Substrate, see https://substrate.dev/
    Substrate,
    /// Hyperledger Burrow, see https://github.com/hyperledger/burrow/
    Burrow
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Target::Substrate => write!(f, "Substrate"),
            Target::Burrow => write!(f, "Burrow")
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
pub fn compile_with_context(ctx: &inkwell::context::Context, src: &str, filename: &str, target: &Target) -> (Option<(Vec<u8>, String)>, Vec<output::Output>) {
    let ast = match parser::parse(src) {
        Ok(s) => s,
        Err(errors) => {
            return (None, errors);
        }
    };

    // resolve
    let (contracts, errors) = resolver::resolver(ast, target);

    if contracts.is_empty() {
        return (None, errors);
    }

    assert_eq!(contracts.len(), 1);

    // abi
    let (abistr, _) = abi::generate_abi(&contracts[0], false);

    // codegen
    let contract = emit::Contract::build(ctx, &contracts[0], filename);

    let obj = contract.wasm("default").expect("llvm wasm emit should work");

    let bc = link::link(&obj, target);

    (Some((bc, abistr)), errors)
}

/// Parse and resolve the Solidity source code provided in src, for the target chain as specified in target.
/// The result is a list of resolved contracts (if successful) and a list of compiler warnings, errors and
/// informational messages like `found contact N`.
///
/// Note that multiple contracts can be specified in on solidity source file.
pub fn parse_and_resolve(src: &str, target: &Target) -> (Vec<resolver::Contract>, Vec<output::Output>) {
    let ast = match parser::parse(src) {
        Ok(s) => s,
        Err(errors) => {
            return (Vec::new(), errors);
        }
    };

    // resolve
    resolver::resolver(ast, target)
}
