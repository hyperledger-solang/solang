
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
extern crate serde;

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
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct EwasmContract {
    pub wasm: String
}

#[derive(Serialize)]
pub struct JsonContract {
    abi: Vec<resolver::ABI>,
    ewasm: EwasmContract
}

#[derive(Serialize)]
pub struct JsonResult {
    pub errors: Vec<output::OutputJson>,
    pub contracts: HashMap<String, HashMap<String, JsonContract>>,
}

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
        .arg(Arg::with_name("JSON")
            .help("mimic solidity json output on stdout")
            .long("standard-json"))
        .arg(Arg::with_name("NOLINK")
            .help("Skip linking, emit wasm object file")
            .long("no-link"))
        .get_matches();

    let mut fatal = false;
    let mut json = JsonResult{
        errors: Vec::new(),
        contracts: HashMap::new(),
    };

    for filename in matches.values_of("INPUT").unwrap() {
        let mut f = File::open(&filename).expect("file not found");

        let mut contents = String::new();
        f.read_to_string(&mut contents)
            .expect("something went wrong reading the file");

        let mut past = match parse::parse(&contents) {
            Ok(s) => s,
            Err(errors) => {
                if matches.is_present("JSON") {
                    let mut out = output::message_as_json(filename, &contents, &errors);
                    json.errors.append(&mut out);
                } else {
                    output::print_messages(filename, &contents, &errors,  matches.is_present("VERBOSE"));
                    fatal = true;
                }
                continue;
            }
        };

        // resolve phase
        let (contracts, errors) = resolver::resolver(past);

        if matches.is_present("JSON") {
            let mut out = output::message_as_json(filename, &contents, &errors);
            json.errors.append(&mut out);
        } else {
            output::print_messages(filename, &contents, &errors,  matches.is_present("VERBOSE"));
        }

        let mut json_contracts = HashMap::new();

        // emit phase
        for contract in &contracts {
            if matches.is_present("CFG") {
                println!("{}\n", contract.to_string());
            }

            let abi = contract.generate_abi();

            let contract = emit::Contract::new(contract, &filename);
            if matches.is_present("JSON") {
                json_contracts.insert(contract.name.to_owned(), JsonContract{
                    abi,
                    ewasm: EwasmContract{
                        wasm: hex::encode_upper(contract.wasm().unwrap())
                    }
                });
                continue;
            } else if matches.is_present("LLVM") {
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

                let wasm_filename = contract.name.to_string() + ".wasm";

                let mut file = File::create(wasm_filename).unwrap();
                file.write_all(&obj).unwrap();


                let abi_filename = contract.name.to_string() + ".abi";

                file = File::create(abi_filename).unwrap();
                file.write_all(serde_json::to_string(&abi).unwrap().as_bytes()).unwrap();
            }
        }

        json.contracts.insert(filename.to_owned(), json_contracts);
    }

    if matches.is_present("JSON") {
        println!("{}", serde_json::to_string(&json).unwrap());
    } else if fatal {
        std::process::exit(1);
    }
}
