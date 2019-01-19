#![feature(box_patterns)]

extern crate lalrpop;
extern crate num_bigint;
extern crate lalrpop_util;
extern crate llvm_sys;
extern crate num_traits;
extern crate parity_wasm;
extern crate wasmi;
extern crate clap;
extern crate lazy_static;

use clap::{App, Arg};
mod ast;
mod solidity;
mod resolve;
mod emit;
mod link;
mod vartable;
mod test;
mod output;
mod parse;

use std::fs::File;
use std::io::prelude::*;
use emit::Emitter;

fn main() {
    let matches = App::new("solang")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Solidity to WASM Compiler")
        .arg(Arg::with_name("INPUT")
            .help("Solidity input files")
            .required(true)
            .multiple(true))
        .arg(Arg::with_name("LLVM")
            .help("emit llvm IR rather than wasm")
            .long("emit-llvm"))
        .get_matches();

    let mut fatal = false;

    for filename in matches.values_of("INPUT").unwrap() {
        let mut f = File::open(&filename).expect("file not found");

        let mut contents = String::new();
        f.read_to_string(&mut contents)
            .expect("something went wrong reading the file");

        let mut past = match parse::parse(&contents) {
            Ok(s) => s,
            Err(errors) => {
                output::print_messages(filename, &contents, &errors);
                fatal = true;
                continue;
            }
        };

        // resolve phase
        let errors = resolve::resolve(&mut past);

        output::print_messages(filename, &contents, &errors);

        if !past.resolved {
            fatal = true;
            continue;
        }

        // emit phase
        let res = Emitter::new(past);

        for contract in &res.contracts {
            if matches.is_present("LLVM") {
                contract.dump_llvm();
            } else {
                if let Err(s) = contract.wasm_file(&res, contract.name.to_string() + ".wasm") {
                    println!("error: {}", s);
                }
            }
        }
    }

    if fatal {
        std::process::exit(1);
    }
}
