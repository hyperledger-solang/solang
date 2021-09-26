use clap::{App, Arg, ArgMatches};
use itertools::Itertools;
use serde::Serialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use solang::abi;
use solang::codegen::{codegen, Options};
use solang::emit::Generate;
use solang::file_resolver::FileResolver;
use solang::sema::{ast::Namespace, diagnostics};

mod doc;
mod languageserver;

#[derive(Serialize)]
pub struct EwasmContract {
    pub wasm: String,
}

#[derive(Serialize)]
pub struct JsonContract {
    abi: Vec<abi::ethereum::ABI>,
    ewasm: EwasmContract,
}

#[derive(Serialize)]
pub struct JsonResult {
    pub errors: Vec<diagnostics::OutputJson>,
    pub contracts: HashMap<String, HashMap<String, JsonContract>>,
}

fn main() {
    let matches = App::new("solang")
        .version(&*format!("version {}", env!("GIT_HASH")))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("INPUT")
                .help("Solidity input files")
                .required(true)
                .conflicts_with("LANGUAGESERVER")
                .multiple(true),
        )
        .arg(
            Arg::with_name("EMIT")
                .help("Emit compiler state at early stage")
                .long("emit")
                .takes_value(true)
                .possible_values(&["ast", "cfg", "llvm-ir", "llvm-bc", "object", "asm"]),
        )
        .arg(
            Arg::with_name("OPT")
                .help("Set llvm optimizer level")
                .short("O")
                .takes_value(true)
                .possible_values(&["none", "less", "default", "aggressive"])
                .default_value("default"),
        )
        .arg(
            Arg::with_name("TARGET")
                .help("Target to build for")
                .long("target")
                .takes_value(true)
                .possible_values(&["solana", "substrate", "ewasm"])
                .default_value("substrate"),
        )
        .arg(
            Arg::with_name("STD-JSON")
                .help("mimic solidity json output on stdout")
                .long("standard-json"),
        )
        .arg(
            Arg::with_name("VERBOSE")
                .help("show debug messages")
                .short("v")
                .long("verbose"),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("output directory")
                .short("o")
                .long("output")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("IMPORTPATH")
                .help("Directory to search for solidity files")
                .short("I")
                .long("importpath")
                .takes_value(true)
                .multiple(true)
                .require_delimiter(true),
        )
        .arg(
            Arg::with_name("IMPORTMAP")
                .help("Map directory to search for solidity files [format: map=path]")
                .short("m")
                .long("importmap")
                .takes_value(true)
                .multiple(true)
                .require_delimiter(true),
        )
        .arg(
            Arg::with_name("CONSTANTFOLDING")
                .help("Disable constant folding codegen optimization")
                .long("no-constant-folding")
                .display_order(1),
        )
        .arg(
            Arg::with_name("STRENGTHREDUCE")
                .help("Disable strength reduce codegen optimization")
                .long("no-strength-reduce")
                .display_order(2),
        )
        .arg(
            Arg::with_name("DEADSTORAGE")
                .help("Disable dead storage codegen optimization")
                .long("no-dead-storage")
                .display_order(3),
        )
        .arg(
            Arg::with_name("VECTORTOSLICE")
                .help("Disable vector to slice codegen optimization")
                .long("no-vector-to-slice")
                .display_order(4),
        )
        .arg(
            Arg::with_name("MATHOVERFLOW")
                .help("Enable math overflow checking")
                .long("math-overflow")
                .display_order(5),
        )
        .arg(
            Arg::with_name("LANGUAGESERVER")
                .help("Start language server on stdin/stdout")
                .conflicts_with_all(&["STD-JSON", "OUTPUT", "EMIT", "OPT", "INPUT"])
                .long("language-server"),
        )
        .arg(
            Arg::with_name("DOC")
                .help("Generate documention for contracts using doc comments")
                .long("doc"),
        )
        .get_matches();

    let target = match matches.value_of("TARGET") {
        Some("solana") => solang::Target::Solana,
        Some("substrate") => solang::Target::Substrate,
        Some("ewasm") => solang::Target::Ewasm,
        _ => unreachable!(),
    };

    if matches.is_present("LANGUAGESERVER") {
        languageserver::start_server(target);
    }

    let verbose = matches.is_present("VERBOSE");
    let mut json = JsonResult {
        errors: Vec::new(),
        contracts: HashMap::new(),
    };

    if verbose {
        eprintln!("info: Solang version {}", env!("GIT_HASH"));
    }

    let math_overflow_check = matches.is_present("MATHOVERFLOW");

    let mut cache = FileResolver::new();

    for filename in matches.values_of("INPUT").unwrap() {
        if let Ok(path) = PathBuf::from(filename).canonicalize() {
            let _ = cache.add_import_path(path.parent().unwrap().to_path_buf());
        }
    }

    if let Err(e) = cache.add_import_path(PathBuf::from(".")) {
        eprintln!(
            "error: cannot add current directory to import path: {}",
            e.to_string()
        );
        std::process::exit(1);
    }

    if let Some(paths) = matches.values_of("IMPORTPATH") {
        for path in paths {
            if let Err(e) = cache.add_import_path(PathBuf::from(path)) {
                eprintln!("error: import path ‘{}’: {}", path, e.to_string());
                std::process::exit(1);
            }
        }
    }

    if let Some(maps) = matches.values_of("IMPORTMAP") {
        for p in maps {
            if let Some((map, path)) = p.split_once('=') {
                if let Err(e) = cache.add_import_map(OsString::from(map), PathBuf::from(path)) {
                    eprintln!("error: import path ‘{}’: {}", path, e.to_string());
                    std::process::exit(1);
                }
            } else {
                eprintln!("error: import map ‘{}’: contains no ‘=’", p);
                std::process::exit(1);
            }
        }
    }

    if matches.is_present("DOC") {
        let verbose = matches.is_present("VERBOSE");
        let mut success = true;
        let mut files = Vec::new();

        for filename in matches.values_of("INPUT").unwrap() {
            let ns = solang::parse_and_resolve(filename, &mut cache, target);

            diagnostics::print_messages(&cache, &ns, verbose);

            if ns.contracts.is_empty() {
                eprintln!("{}: error: no contracts found", filename);
                success = false;
            } else if diagnostics::any_errors(&ns.diagnostics) {
                success = false;
            } else {
                files.push(ns);
            }
        }

        if success {
            // generate docs
            doc::generate_docs(matches.value_of("OUTPUT").unwrap_or("."), &files, verbose);
        }
    } else {
        let opt_level = match matches.value_of("OPT").unwrap() {
            "none" => inkwell::OptimizationLevel::None,
            "less" => inkwell::OptimizationLevel::Less,
            "default" => inkwell::OptimizationLevel::Default,
            "aggressive" => inkwell::OptimizationLevel::Aggressive,
            _ => unreachable!(),
        };

        let opt = Options {
            dead_storage: !matches.is_present("DEADSTORAGE"),
            strength_reduce: !matches.is_present("STRENGTHREDUCE"),
            constant_folding: !matches.is_present("CONSTANTFOLDING"),
            vector_to_slice: !matches.is_present("VECTORTOSLICE"),
            math_overflow_check,
            opt_level,
        };

        let mut namespaces = Vec::new();

        let mut errors = false;

        for filename in matches.values_of("INPUT").unwrap() {
            match process_filename(filename, &mut cache, target, &matches, &mut json, &opt) {
                Ok(ns) => namespaces.push(ns),
                Err(_) => {
                    errors = true;
                }
            }
        }

        if errors {
            eprintln!("error: not all contracts are valid");
            std::process::exit(1);
        }

        if target == solang::Target::Solana {
            let context = inkwell::context::Context::create();

            let binary = solang::compile_many(
                &context,
                &namespaces,
                "bundle.sol",
                opt_level,
                math_overflow_check,
            );

            if !save_intermediates(&binary, &matches) {
                let bin_filename = output_file(&matches, "bundle", target.file_extension());

                if matches.is_present("VERBOSE") {
                    eprintln!(
                        "info: Saving binary {} for contracts: {}",
                        bin_filename.display(),
                        namespaces
                            .iter()
                            .flat_map(|ns| {
                                ns.contracts.iter().filter_map(|contract| {
                                    if contract.is_concrete() {
                                        Some(contract.name.as_str())
                                    } else {
                                        None
                                    }
                                })
                            })
                            .sorted()
                            .dedup()
                            .join(", "),
                    );
                }

                let code = binary
                    .code(Generate::Linked)
                    .expect("llvm code emit should work");

                let mut file = match File::create(&bin_filename) {
                    Ok(file) => file,
                    Err(err) => {
                        eprintln!(
                            "error: cannot create file ‘{}’: {}",
                            bin_filename.display(),
                            err,
                        );
                        std::process::exit(1);
                    }
                };
                file.write_all(&code).unwrap();

                // Write all ABI files
                for ns in &namespaces {
                    for contract_no in 0..ns.contracts.len() {
                        let contract = &ns.contracts[contract_no];

                        if !contract.is_concrete() {
                            continue;
                        }

                        let (abi_bytes, abi_ext) =
                            abi::generate_abi(contract_no, ns, &code, verbose);
                        let abi_filename = output_file(&matches, &contract.name, abi_ext);

                        if verbose {
                            eprintln!(
                                "info: Saving ABI {} for contract {}",
                                abi_filename.display(),
                                contract.name
                            );
                        }

                        let mut file = match File::create(abi_filename) {
                            Ok(file) => file,
                            Err(err) => {
                                eprintln!(
                                    "error: cannot create file ‘{}’: {}",
                                    bin_filename.display(),
                                    err
                                );
                                std::process::exit(1);
                            }
                        };

                        file.write_all(abi_bytes.as_bytes()).unwrap();
                    }
                }
            }
        }

        if matches.is_present("STD-JSON") {
            println!("{}", serde_json::to_string(&json).unwrap());
        }
    }
}

fn output_file(matches: &ArgMatches, stem: &str, ext: &str) -> PathBuf {
    Path::new(matches.value_of("OUTPUT").unwrap_or(".")).join(format!("{}.{}", stem, ext))
}

fn process_filename(
    filename: &str,
    resolver: &mut FileResolver,
    target: solang::Target,
    matches: &ArgMatches,
    json: &mut JsonResult,
    opt: &Options,
) -> Result<Namespace, ()> {
    let verbose = matches.is_present("VERBOSE");

    let mut json_contracts = HashMap::new();

    // resolve phase
    let mut ns = solang::parse_and_resolve(filename, resolver, target);

    // codegen all the contracts; some additional errors/warnings will be detected here
    codegen(&mut ns, opt);

    if matches.is_present("STD-JSON") {
        let mut out = diagnostics::message_as_json(&ns, resolver);
        json.errors.append(&mut out);
    } else {
        diagnostics::print_messages(resolver, &ns, verbose);
    }

    if ns.contracts.is_empty() || diagnostics::any_errors(&ns.diagnostics) {
        return Err(());
    }

    if let Some("ast") = matches.value_of("EMIT") {
        println!("{}", ns.print(filename));
        return Ok(ns);
    }

    // emit phase
    for contract_no in 0..ns.contracts.len() {
        let resolved_contract = &ns.contracts[contract_no];

        if !resolved_contract.is_concrete() {
            continue;
        }

        if let Some("cfg") = matches.value_of("EMIT") {
            println!("{}", resolved_contract.print_cfg(&ns));
            continue;
        }

        if target == solang::Target::Solana {
            if verbose {
                eprintln!(
                    "info: contract {} uses at least {} bytes account data",
                    resolved_contract.name, resolved_contract.fixed_layout_size,
                );
            }
            // we don't generate llvm here; this is done in one go for all contracts
            continue;
        }

        if verbose {
            eprintln!(
                "info: Generating LLVM IR for contract {} with target {}",
                resolved_contract.name, ns.target
            );
        }

        let context = inkwell::context::Context::create();

        let binary = resolved_contract.emit(
            &ns,
            &context,
            filename,
            opt.opt_level,
            opt.math_overflow_check,
        );

        if save_intermediates(&binary, matches) {
            continue;
        }

        if matches.is_present("STD-JSON") {
            json_contracts.insert(
                binary.name.to_owned(),
                JsonContract {
                    abi: abi::ethereum::gen_abi(contract_no, &ns),
                    ewasm: EwasmContract {
                        wasm: hex::encode_upper(&resolved_contract.code),
                    },
                },
            );
        } else if target != solang::Target::Solana {
            let bin_filename = output_file(matches, &binary.name, target.file_extension());

            if verbose {
                eprintln!(
                    "info: Saving binary {} for contract {}",
                    bin_filename.display(),
                    binary.name
                );
            }

            let mut file = match File::create(&bin_filename) {
                Ok(file) => file,
                Err(err) => {
                    eprintln!(
                        "error: cannot create file ‘{}’: {}",
                        bin_filename.display(),
                        err,
                    );
                    std::process::exit(1);
                }
            };
            file.write_all(&resolved_contract.code).unwrap();

            let (abi_bytes, abi_ext) =
                abi::generate_abi(contract_no, &ns, &resolved_contract.code, verbose);
            let abi_filename = output_file(matches, &binary.name, abi_ext);

            if verbose {
                eprintln!(
                    "info: Saving ABI {} for contract {}",
                    abi_filename.display(),
                    binary.name
                );
            }

            let mut file = match File::create(&abi_filename) {
                Ok(file) => file,
                Err(err) => {
                    eprintln!(
                        "error: cannot create file ‘{}’: {}",
                        abi_filename.display(),
                        err,
                    );
                    std::process::exit(1);
                }
            };
            file.write_all(abi_bytes.as_bytes()).unwrap();
        }
    }

    json.contracts.insert(filename.to_owned(), json_contracts);

    Ok(ns)
}

fn save_intermediates(binary: &solang::emit::Binary, matches: &ArgMatches) -> bool {
    let verbose = matches.is_present("VERBOSE");

    match matches.value_of("EMIT") {
        Some("llvm-ir") => {
            if let Some(runtime) = &binary.runtime {
                // In Ethereum, an ewasm contract has two parts, deployer and runtime. The deployer code returns the runtime wasm
                // as a byte string
                let llvm_filename = output_file(matches, &format!("{}_deploy", binary.name), "ll");

                if verbose {
                    eprintln!(
                        "info: Saving deployer LLVM {} for contract {}",
                        llvm_filename.display(),
                        binary.name
                    );
                }

                binary.dump_llvm(&llvm_filename).unwrap();

                let llvm_filename = output_file(matches, &format!("{}_runtime", binary.name), "ll");

                if verbose {
                    eprintln!(
                        "info: Saving runtime LLVM {} for contract {}",
                        llvm_filename.display(),
                        binary.name
                    );
                }

                runtime.dump_llvm(&llvm_filename).unwrap();
            } else {
                let llvm_filename = output_file(matches, &binary.name, "ll");

                if verbose {
                    eprintln!(
                        "info: Saving LLVM IR {} for contract {}",
                        llvm_filename.display(),
                        binary.name
                    );
                }

                binary.dump_llvm(&llvm_filename).unwrap();
            }

            true
        }

        Some("llvm-bc") => {
            // In Ethereum, an ewasm contract has two parts, deployer and runtime. The deployer code returns the runtime wasm
            // as a byte string
            if let Some(runtime) = &binary.runtime {
                let bc_filename = output_file(matches, &format!("{}_deploy", binary.name), "bc");

                if verbose {
                    eprintln!(
                        "info: Saving deploy LLVM BC {} for contract {}",
                        bc_filename.display(),
                        binary.name
                    );
                }

                binary.bitcode(&bc_filename);

                let bc_filename = output_file(matches, &format!("{}_runtime", binary.name), "bc");

                if verbose {
                    eprintln!(
                        "info: Saving runtime LLVM BC {} for contract {}",
                        bc_filename.display(),
                        binary.name
                    );
                }

                runtime.bitcode(&bc_filename);
            } else {
                let bc_filename = output_file(matches, &binary.name, "bc");

                if verbose {
                    eprintln!(
                        "info: Saving LLVM BC {} for contract {}",
                        bc_filename.display(),
                        binary.name
                    );
                }

                binary.bitcode(&bc_filename);
            }

            true
        }

        Some("object") => {
            let obj = match binary.code(Generate::Object) {
                Ok(o) => o,
                Err(s) => {
                    println!("error: {}", s);
                    std::process::exit(1);
                }
            };

            let obj_filename = output_file(matches, &binary.name, "o");

            if verbose {
                eprintln!(
                    "info: Saving Object {} for contract {}",
                    obj_filename.display(),
                    binary.name
                );
            }

            let mut file = File::create(obj_filename).unwrap();
            file.write_all(&obj).unwrap();
            true
        }
        Some("asm") => {
            let obj = match binary.code(Generate::Assembly) {
                Ok(o) => o,
                Err(s) => {
                    println!("error: {}", s);
                    std::process::exit(1);
                }
            };

            let obj_filename = output_file(matches, &binary.name, "asm");

            if verbose {
                eprintln!(
                    "info: Saving Assembly {} for contract {}",
                    obj_filename.display(),
                    binary.name
                );
            }

            let mut file = File::create(obj_filename).unwrap();
            file.write_all(&obj).unwrap();
            true
        }
        Some("cfg") => true,
        Some("ast") => true,
        _ => false,
    }
}
