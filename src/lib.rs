extern crate blake2_rfc;
extern crate clap;
extern crate hex;
extern crate inkwell;
extern crate lalrpop_util;
extern crate lazy_static;
extern crate num_bigint;
extern crate num_derive;
extern crate num_traits;
extern crate parity_wasm;
extern crate serde;
extern crate serde_derive;
extern crate tiny_keccak;
extern crate unescape;

pub mod abi;
pub mod link;
pub mod output;

mod emit;
mod parser;
mod resolver;

use inkwell::OptimizationLevel;
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
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Target::Substrate => write!(f, "Substrate"),
            Target::Ewasm => write!(f, "ewasm"),
            Target::Sabre => write!(f, "Sawtooth Sabre"),
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
    src: &str,
    filename: &str,
    opt: OptimizationLevel,
    target: Target,
) -> (Vec<(Vec<u8>, String)>, Vec<output::Output>) {
    let ctx = inkwell::context::Context::create();

    let ast = match parser::parse(src) {
        Ok(s) => s,
        Err(errors) => {
            return (Vec::new(), errors);
        }
    };

    // resolve
    let (ns, errors) = resolver::resolver(ast, target);

    let results = ns
        .contracts
        .iter()
        .map(|c| {
            let (abistr, _) = abi::generate_abi(c, &ns, false);

            // codegen
            let contract = emit::Contract::build(&ctx, c, &ns, filename, opt);

            let obj = contract.wasm().expect("llvm wasm emit should work");

            let bc = link::link(&obj, target);

            (bc, abistr)
        })
        .collect();

    (results, errors)
}

/// Parse and resolve the Solidity source code provided in src, for the target chain as specified in target.
/// The result is a list of resolved contracts (if successful) and a list of compiler warnings, errors and
/// informational messages like `found contact N`.
///
/// Note that multiple contracts can be specified in on solidity source file.
pub fn parse_and_resolve(src: &str, target: Target) -> (resolver::Namespace, Vec<output::Output>) {
    let ast = match parser::parse(src) {
        Ok(s) => s,
        Err(errors) => {
            return (resolver::Namespace::new(target), errors);
        }
    };

    // resolve
    resolver::resolver(ast, target)
}
