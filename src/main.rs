
extern crate lalrpop;
extern crate num_bigint;
extern crate lalrpop_util;
extern crate llvm_sys;
extern crate num_traits;
extern crate parity_wasm;
extern crate wasmi;
extern crate clap;
extern crate lazy_static;
extern crate hex;
extern crate unescape;
extern crate tiny_keccak;

use clap::{App, Arg};
mod ast;
mod solidity;
mod resolver;
mod emit;
mod link;
mod test;
mod output;
mod parse;
mod cfg;

use std::fs::File;
use std::io::prelude::*;

fn main() {
    let matches = App::new("solang")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Solidity to WASM Compiler")
        .arg(Arg::with_name("INPUT")
            .help("Solidity input files")
            .required(true)
            .multiple(true))
        .arg(Arg::with_name("CFG")
            .help("emit control flow graph")
            .long("emit-cfg"))
        .arg(Arg::with_name("VERBOSE")
            .help("show verbose messages")
            .short("v")
            .long("verbose"))
        .arg(Arg::with_name("LLVM")
            .help("emit llvm IR rather than wasm")
            .long("emit-llvm"))
        .arg(Arg::with_name("NOLINK")
            .help("Skip linking, emit wasm object file")
            .long("no-link"))
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
                output::print_messages(filename, &contents, &errors,  matches.is_present("VERBOSE"));
                fatal = true;
                continue;
            }
        };

        // resolve phase
        let (contracts, errors) = resolver::resolver(past);

        output::print_messages(filename, &contents, &errors, matches.is_present("VERBOSE"));

        // emit phase
        for contract in &contracts {
            if matches.is_present("CFG") {
                println!("{}\n", contract.to_string());
            }

            let contract = emit::Contract::new(contract, &filename);
            if matches.is_present("LLVM") {
                contract.dump_llvm();
            } else {
                let mut obj = match contract.wasm() {
                    Ok(o) => o,
                    Err(s) => {
                        println!("error: {}", s);
                        std::process::exit(1);
                    }
                };

                if !matches.is_present("NOLINK") {
                    obj = link::link(&obj);
                }

                let filename = contract.name.to_string() + ".wasm";

                let mut file = File::create(filename).unwrap();
                file.write_all(&obj).unwrap();
            }
        }
    }

    if fatal {
        std::process::exit(1);
    }
}
