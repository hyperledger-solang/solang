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

#[derive(PartialEq, Clone)]
pub enum Target {
    Substrate,
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

