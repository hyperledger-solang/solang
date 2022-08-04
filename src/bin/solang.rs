// SPDX-License-Identifier: Apache-2.0

use clap::{
    builder::{ArgAction, ValueParser},
    value_parser, Arg, ArgMatches, Command, ValueSource,
};
use itertools::Itertools;
use num_traits::cast::ToPrimitive;
use serde::Serialize;
use solang::{
    abi,
    codegen::{codegen, OptimizationLevel, Options},
    emit::Generate,
    file_resolver::FileResolver,
    sema::{ast::Namespace, diagnostics},
};
use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs::{create_dir_all, File},
    io::prelude::*,
    path::{Path, PathBuf},
};

mod doc;
mod languageserver;

#[derive(Serialize)]
pub struct EwasmContract {
    pub wasm: String,
}

#[derive(Serialize)]
pub struct JsonContract {
    abi: Vec<abi::ethereum::ABI>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ewasm: Option<EwasmContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minimum_space: Option<u32>,
}

#[derive(Serialize)]
pub struct JsonResult {
    pub errors: Vec<diagnostics::OutputJson>,
    pub target: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub program: String,
    pub contracts: HashMap<String, HashMap<String, JsonContract>>,
}

fn main() {
    let matches = Command::new("solang")
        .version(&*format!("version {}", env!("SOLANG_VERSION")))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("INPUT")
                .help("Solidity input files")
                .required(true)
                .conflicts_with("LANGUAGESERVER")
                .value_parser(ValueParser::os_string())
                .multiple_values(true),
        )
        .arg(
            Arg::new("EMIT")
                .help("Emit compiler state at early stage")
                .long("emit")
                .takes_value(true)
                .value_parser(["ast-dot", "cfg", "llvm-ir", "llvm-bc", "object", "asm"]),
        )
        .arg(
            Arg::new("OPT")
                .help("Set llvm optimizer level")
                .short('O')
                .takes_value(true)
                .value_parser(["none", "less", "default", "aggressive"])
                .default_value("default"),
        )
        .arg(
            Arg::new("TARGET")
                .help("Target to build for")
                .long("target")
                .takes_value(true)
                .value_parser(["solana", "substrate", "ewasm"])
                .required(true),
        )
        .arg(
            Arg::new("ADDRESS_LENGTH")
                .help("Address length on Substrate")
                .long("address-length")
                .takes_value(true)
                .value_parser(value_parser!(u64).range(4..1024))
                .default_value("32"),
        )
        .arg(
            Arg::new("VALUE_LENGTH")
                .help("Value length on Substrate")
                .long("value-length")
                .value_parser(value_parser!(u64).range(4..1024))
                .takes_value(true)
                .default_value("16"),
        )
        .arg(
            Arg::new("STD-JSON")
                .help("mimic solidity json output on stdout")
                .conflicts_with_all(&["VERBOSE", "OUTPUT", "EMIT"])
                .long("standard-json"),
        )
        .arg(
            Arg::new("VERBOSE")
                .help("show debug messages")
                .short('v')
                .long("verbose"),
        )
        .arg(
            Arg::new("OUTPUT")
                .help("output directory")
                .short('o')
                .long("output")
                .takes_value(true),
        )
        .arg(
            Arg::new("IMPORTPATH")
                .help("Directory to search for solidity files")
                .short('I')
                .long("importpath")
                .takes_value(true)
                .value_parser(ValueParser::path_buf())
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("IMPORTMAP")
                .help("Map directory to search for solidity files [format: map=path]")
                .short('m')
                .long("importmap")
                .takes_value(true)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("CONSTANTFOLDING")
                .help("Disable constant folding codegen optimization")
                .long("no-constant-folding")
                .action(ArgAction::SetFalse)
                .display_order(1),
        )
        .arg(
            Arg::new("STRENGTHREDUCE")
                .help("Disable strength reduce codegen optimization")
                .long("no-strength-reduce")
                .action(ArgAction::SetFalse)
                .display_order(2),
        )
        .arg(
            Arg::new("DEADSTORAGE")
                .help("Disable dead storage codegen optimization")
                .long("no-dead-storage")
                .action(ArgAction::SetFalse)
                .display_order(3),
        )
        .arg(
            Arg::new("VECTORTOSLICE")
                .help("Disable vector to slice codegen optimization")
                .long("no-vector-to-slice")
                .action(ArgAction::SetFalse)
                .display_order(4),
        )
        .arg(
            Arg::new("COMMONSUBEXPRESSIONELIMINATION")
                .help("Disable common subexpression elimination")
                .long("no-cse")
                .action(ArgAction::SetFalse)
                .display_order(5),
        )
        .arg(
            Arg::new("MATHOVERFLOW")
                .help("Enable math overflow checking")
                .long("math-overflow")
                .display_order(6),
        )
        .arg(
            Arg::new("LANGUAGESERVER")
                .help("Start language server on stdin/stdout")
                .conflicts_with_all(&["STD-JSON", "OUTPUT", "EMIT", "OPT", "INPUT"])
                .long("language-server"),
        )
        .arg(
            Arg::new("DOC")
                .help("Generate documention for contracts using doc comments")
                .long("doc"),
        )
        .get_matches();

    let address_length = matches.get_one::<u64>("ADDRESS_LENGTH").unwrap();

    let value_length = matches.get_one::<u64>("VALUE_LENGTH").unwrap();

    let target = match matches.get_one::<String>("TARGET").unwrap().as_str() {
        "solana" => solang::Target::Solana,
        "substrate" => solang::Target::Substrate {
            address_length: *address_length as usize,
            value_length: *value_length as usize,
        },
        "ewasm" => solang::Target::Ewasm,
        _ => unreachable!(),
    };

    if !target.is_substrate()
        && matches.value_source("ADDRESS_LENGTH") == Some(ValueSource::CommandLine)
    {
        eprintln!(
            "error: address length cannot be modified for target '{}'",
            target
        );
        std::process::exit(1);
    }

    if !target.is_substrate()
        && matches.value_source("VALUE_LENGTH") == Some(ValueSource::CommandLine)
    {
        eprintln!(
            "error: value length cannot be modified for target '{}'",
            target
        );
        std::process::exit(1);
    }

    if matches.contains_id("LANGUAGESERVER") {
        languageserver::start_server(target, matches);
    }

    let verbose = matches.contains_id("VERBOSE");
    let mut json = JsonResult {
        errors: Vec::new(),
        target: target.to_string(),
        program: String::new(),
        contracts: HashMap::new(),
    };

    if verbose {
        eprintln!("info: Solang version {}", env!("SOLANG_VERSION"));
    }

    let math_overflow_check = matches.contains_id("MATHOVERFLOW");

    let mut resolver = FileResolver::new();

    for filename in matches.get_many::<OsString>("INPUT").unwrap() {
        if let Ok(path) = PathBuf::from(filename).canonicalize() {
            let _ = resolver.add_import_path(path.parent().unwrap());
        }
    }

    if let Err(e) = resolver.add_import_path(&PathBuf::from(".")) {
        eprintln!("error: cannot add current directory to import path: {}", e);
        std::process::exit(1);
    }

    if let Some(paths) = matches.get_many::<PathBuf>("IMPORTPATH") {
        for path in paths {
            if let Err(e) = resolver.add_import_path(path) {
                eprintln!("error: import path '{}': {}", path.to_string_lossy(), e);
                std::process::exit(1);
            }
        }
    }

    if let Some(maps) = matches.get_many::<String>("IMPORTMAP") {
        for p in maps {
            if let Some((map, path)) = p.split_once('=') {
                if let Err(e) = resolver.add_import_map(OsString::from(map), PathBuf::from(path)) {
                    eprintln!("error: import path '{}': {}", path, e);
                    std::process::exit(1);
                }
            } else {
                eprintln!("error: import map '{}': contains no '='", p);
                std::process::exit(1);
            }
        }
    }

    if matches.contains_id("DOC") {
        let verbose = matches.contains_id("VERBOSE");
        let mut success = true;
        let mut files = Vec::new();

        for filename in matches.get_many::<&OsString>("INPUT").unwrap() {
            let ns = solang::parse_and_resolve(filename, &mut resolver, target);

            ns.print_diagnostics(&resolver, verbose);

            if ns.contracts.is_empty() {
                eprintln!("{}: error: no contracts found", filename.to_string_lossy());
                success = false;
            } else if ns.diagnostics.any_errors() {
                success = false;
            } else {
                files.push(ns);
            }
        }

        if success {
            // generate docs
            doc::generate_docs(
                matches
                    .get_one::<String>("OUTPUT")
                    .unwrap_or(&String::from(".")),
                &files,
                verbose,
            );
        }
    } else {
        let opt_level = match matches.get_one::<String>("OPT").unwrap().as_str() {
            "none" => OptimizationLevel::None,
            "less" => OptimizationLevel::Less,
            "default" => OptimizationLevel::Default,
            "aggressive" => OptimizationLevel::Aggressive,
            _ => unreachable!(),
        };

        let opt = Options {
            dead_storage: *matches.get_one::<bool>("DEADSTORAGE").unwrap(),
            constant_folding: *matches.get_one::<bool>("CONSTANTFOLDING").unwrap(),
            strength_reduce: *matches.get_one::<bool>("STRENGTHREDUCE").unwrap(),
            vector_to_slice: *matches.get_one::<bool>("VECTORTOSLICE").unwrap(),
            math_overflow_check,
            common_subexpression_elimination: *matches
                .get_one::<bool>("COMMONSUBEXPRESSIONELIMINATION")
                .unwrap(),
            opt_level,
        };

        let mut namespaces = Vec::new();

        let mut errors = false;

        for filename in matches.get_many::<OsString>("INPUT").unwrap() {
            match process_file(filename, &mut resolver, target, &matches, &mut json, &opt) {
                Ok(ns) => namespaces.push(ns),
                Err(_) => {
                    errors = true;
                }
            }
        }

        let namespaces = namespaces.iter().collect::<Vec<_>>();

        if let Some("ast-dot") = matches.get_one::<String>("EMIT").map(|v| v.as_str()) {
            std::process::exit(0);
        }

        if errors {
            if matches.contains_id("STD-JSON") {
                println!("{}", serde_json::to_string(&json).unwrap());
                std::process::exit(0);
            } else {
                eprintln!("error: not all contracts are valid");
                std::process::exit(1);
            }
        }

        if target == solang::Target::Solana {
            let context = inkwell::context::Context::create();

            let binary = solang::compile_many(
                &context,
                &namespaces,
                "bundle.sol",
                opt_level.into(),
                math_overflow_check,
            );

            if !save_intermediates(&binary, &matches) {
                let bin_filename = output_file(&matches, "bundle", target.file_extension());

                if matches.contains_id("VERBOSE") {
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

                if matches.contains_id("STD-JSON") {
                    json.program = hex::encode_upper(&code);
                } else {
                    let mut file = create_file(&bin_filename);
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

                            let mut file = create_file(&abi_filename);

                            file.write_all(abi_bytes.as_bytes()).unwrap();
                        }
                    }
                }
            }
        }

        if matches.contains_id("STD-JSON") {
            println!("{}", serde_json::to_string(&json).unwrap());
        }
    }
}

fn output_file(matches: &ArgMatches, stem: &str, ext: &str) -> PathBuf {
    Path::new(
        matches
            .get_one::<String>("OUTPUT")
            .unwrap_or(&String::from(".")),
    )
    .join(format!("{}.{}", stem, ext))
}

fn process_file(
    filename: &OsStr,
    resolver: &mut FileResolver,
    target: solang::Target,
    matches: &ArgMatches,
    json: &mut JsonResult,
    opt: &Options,
) -> Result<Namespace, ()> {
    let verbose = matches.contains_id("VERBOSE");

    let mut json_contracts = HashMap::new();

    // resolve phase
    let mut ns = solang::parse_and_resolve(filename, resolver, target);

    // codegen all the contracts; some additional errors/warnings will be detected here
    codegen(&mut ns, opt);

    if matches.contains_id("STD-JSON") {
        let mut out = ns.diagnostics_as_json(resolver);
        json.errors.append(&mut out);
    } else {
        ns.print_diagnostics(resolver, verbose);
    }

    if let Some("ast-dot") = matches.get_one::<String>("EMIT").map(|v| v.as_str()) {
        let filepath = PathBuf::from(filename);
        let stem = filepath.file_stem().unwrap().to_string_lossy();
        let dot_filename = output_file(matches, &stem, "dot");

        if verbose {
            eprintln!("info: Saving graphviz dot {}", dot_filename.display());
        }

        let dot = ns.dotgraphviz();

        let mut file = create_file(&dot_filename);

        if let Err(err) = file.write_all(dot.as_bytes()) {
            eprintln!("{}: error: {}", dot_filename.display(), err);
            std::process::exit(1);
        }

        return Ok(ns);
    }

    if ns.contracts.is_empty() || ns.diagnostics.any_errors() {
        return Err(());
    }

    // emit phase
    for contract_no in 0..ns.contracts.len() {
        let resolved_contract = &ns.contracts[contract_no];

        if !resolved_contract.is_concrete() {
            continue;
        }

        if let Some("cfg") = matches.get_one::<String>("EMIT").map(|v| v.as_str()) {
            println!("{}", resolved_contract.print_cfg(&ns));
            continue;
        }

        if target == solang::Target::Solana {
            if matches.contains_id("STD-JSON") {
                json_contracts.insert(
                    resolved_contract.name.to_owned(),
                    JsonContract {
                        abi: abi::ethereum::gen_abi(contract_no, &ns),
                        ewasm: None,
                        minimum_space: Some(resolved_contract.fixed_layout_size.to_u32().unwrap()),
                    },
                );
            }

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
        let filename_string = filename.to_string_lossy();

        let binary = resolved_contract.emit(
            &ns,
            &context,
            &filename_string,
            opt.opt_level.into(),
            opt.math_overflow_check,
        );

        if save_intermediates(&binary, matches) {
            continue;
        }

        if matches.contains_id("STD-JSON") {
            json_contracts.insert(
                binary.name.to_owned(),
                JsonContract {
                    abi: abi::ethereum::gen_abi(contract_no, &ns),
                    ewasm: Some(EwasmContract {
                        wasm: hex::encode_upper(&resolved_contract.code),
                    }),
                    minimum_space: None,
                },
            );
        } else {
            let bin_filename = output_file(matches, &binary.name, target.file_extension());

            if verbose {
                eprintln!(
                    "info: Saving binary {} for contract {}",
                    bin_filename.display(),
                    binary.name
                );
            }

            let mut file = create_file(&bin_filename);
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

            let mut file = create_file(&abi_filename);
            file.write_all(abi_bytes.as_bytes()).unwrap();
        }
    }

    json.contracts
        .insert(filename.to_string_lossy().to_string(), json_contracts);

    Ok(ns)
}

fn save_intermediates(binary: &solang::emit::binary::Binary, matches: &ArgMatches) -> bool {
    let verbose = matches.contains_id("VERBOSE");

    match matches.get_one::<String>("EMIT").map(|v| v.as_str()) {
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

            let mut file = create_file(&obj_filename);
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

            let mut file = create_file(&obj_filename);
            file.write_all(&obj).unwrap();
            true
        }
        Some("cfg") => true,
        Some("ast-dot") => true,
        _ => false,
    }
}

fn create_file(path: &Path) -> File {
    if let Some(parent) = path.parent() {
        if let Err(err) = create_dir_all(parent) {
            eprintln!(
                "error: cannot create output directory '{}': {}",
                parent.display(),
                err
            );
            std::process::exit(1);
        }
    }

    match File::create(path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("error: cannot create file '{}': {}", path.display(), err,);
            std::process::exit(1);
        }
    }
}
