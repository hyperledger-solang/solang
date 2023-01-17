// SPDX-License-Identifier: Apache-2.0

use clap::{
    builder::{ArgAction, ValueParser},
    parser::ValueSource,
    value_parser, Arg, ArgMatches, Command,
};
use clap_complete::{generate, Shell};
use solang::{
    abi,
    codegen::{codegen, OptimizationLevel, Options},
    emit::Generate,
    file_resolver::FileResolver,
    sema::ast::Namespace,
    standard_json::{EwasmContract, JsonContract, JsonResult},
    Target,
};
use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs::{create_dir_all, File},
    io::prelude::*,
    path::{Path, PathBuf},
    process::exit,
};

mod doc;
mod idl;
mod languageserver;

fn main() {
    let version: &'static str = concat!("version ", env!("SOLANG_VERSION"));

    let app = || {
        Command::new("solang")
            .version(version)
            .author(env!("CARGO_PKG_AUTHORS"))
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .subcommand_required(true)
            .subcommand(
                Command::new("compile")
                    .about("Compile Solidity source files")
                    .arg(
                        Arg::new("INPUT")
                            .help("Solidity input files")
                            .required(true)
                            .value_parser(ValueParser::os_string())
                            .num_args(1..),
                    )
                    .arg(
                        Arg::new("EMIT")
                            .help("Emit compiler state at early stage")
                            .long("emit")
                            .num_args(1)
                            .value_parser([
                                "ast-dot", "cfg", "llvm-ir", "llvm-bc", "object", "asm",
                            ]),
                    )
                    .arg(
                        Arg::new("OPT")
                            .help("Set llvm optimizer level")
                            .short('O')
                            .num_args(1)
                            .value_parser(["none", "less", "default", "aggressive"])
                            .default_value("default"),
                    )
                    .arg(
                        Arg::new("TARGET")
                            .help("Target to build for [possible values: solana, substrate, soroban]")
                            .long("target")
                            .num_args(1)
                            .value_parser(["solana", "substrate", "soroban", "evm"])
                            .hide_possible_values(true)
                            .required(true),
                    )
                    .arg(
                        Arg::new("ADDRESS_LENGTH")
                            .help("Address length on Substrate")
                            .long("address-length")
                            .num_args(1)
                            .value_parser(value_parser!(u64).range(4..1024))
                            .default_value("32"),
                    )
                    .arg(
                        Arg::new("VALUE_LENGTH")
                            .help("Value length on Substrate")
                            .long("value-length")
                            .value_parser(value_parser!(u64).range(4..1024))
                            .num_args(1)
                            .default_value("16"),
                    )
                    .arg(
                        Arg::new("STD-JSON")
                            .help("mimic solidity json output on stdout")
                            .conflicts_with_all(["VERBOSE", "OUTPUT", "EMIT"])
                            .long("standard-json"),
                    )
                    .arg(
                        Arg::new("VERBOSE")
                            .help("show debug messages")
                            .short('v')
                            .action(ArgAction::SetTrue)
                            .long("verbose"),
                    )
                    .arg(
                        Arg::new("OUTPUT")
                            .help("output directory")
                            .short('o')
                            .long("output")
                            .num_args(1)
                            .value_parser(ValueParser::os_string()),
                    )
                    .arg(
                        Arg::new("IMPORTPATH")
                            .help("Directory to search for solidity files")
                            .short('I')
                            .long("importpath")
                            .num_args(1)
                            .value_parser(ValueParser::path_buf())
                            .action(ArgAction::Append),
                    )
                    .arg(
                        Arg::new("IMPORTMAP")
                            .help("Map directory to search for solidity files [format: map=path]")
                            .short('m')
                            .long("importmap")
                            .num_args(1)
                            .value_parser(ValueParser::new(parse_import_map))
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
                        Arg::new("LOGAPIRETURNS")
                            .help("Log the return codes of runtime API calls in the environment")
                            .long("log-api-return-codes")
                            .action(ArgAction::SetTrue),
                    )
                    .arg(
                        Arg::new("GENERATEDEBUGINFORMATION")
                            .help("Enable generating debug information for LLVM IR")
                            .short('g')
                            .long("generate-debug-info")
                            .hide(true),
                    ),
            )
            .subcommand(
                Command::new("doc")
                    .about("Generate documention for contracts using doc comments")
                    .arg(
                        Arg::new("INPUT")
                            .help("Solidity input files")
                            .required(true)
                            .value_parser(ValueParser::os_string())
                            .num_args(1..),
                    )
                    .arg(
                        Arg::new("TARGET")
                            .help("Target to build for")
                            .long("target")
                            .num_args(1)
                            .value_parser(["solana", "substrate", "evm"])
                            .required(true),
                    )
                    .arg(
                        Arg::new("ADDRESS_LENGTH")
                            .help("Address length on Substrate")
                            .long("address-length")
                            .num_args(1)
                            .value_parser(value_parser!(u64).range(4..1024))
                            .default_value("32"),
                    )
                    .arg(
                        Arg::new("VALUE_LENGTH")
                            .help("Value length on Substrate")
                            .long("value-length")
                            .value_parser(value_parser!(u64).range(4..1024))
                            .num_args(1)
                            .default_value("16"),
                    )
                    .arg(
                        Arg::new("IMPORTPATH")
                            .help("Directory to search for solidity files")
                            .short('I')
                            .long("importpath")
                            .num_args(1)
                            .value_parser(ValueParser::path_buf())
                            .action(ArgAction::Append),
                    )
                    .arg(
                        Arg::new("IMPORTMAP")
                            .help("Map directory to search for solidity files [format: map=path]")
                            .short('m')
                            .long("importmap")
                            .num_args(1)
                            .value_parser(ValueParser::new(parse_import_map))
                            .action(ArgAction::Append),
                    ),
            )
            .subcommand(
                Command::new("language-server")
                    .about("Start LSP language server on stdin/stdout")
                    .arg(
                        Arg::new("TARGET")
                            .help("Target to build for")
                            .long("target")
                            .num_args(1)
                            .value_parser(["solana", "substrate", "evm"])
                            .required(true),
                    )
                    .arg(
                        Arg::new("ADDRESS_LENGTH")
                            .help("Address length on Substrate")
                            .long("address-length")
                            .num_args(1)
                            .value_parser(value_parser!(u64).range(4..1024))
                            .default_value("32"),
                    )
                    .arg(
                        Arg::new("VALUE_LENGTH")
                            .help("Value length on Substrate")
                            .long("value-length")
                            .value_parser(value_parser!(u64).range(4..1024))
                            .num_args(1)
                            .default_value("16"),
                    )
                    .arg(
                        Arg::new("IMPORTPATH")
                            .help("Directory to search for solidity files")
                            .short('I')
                            .long("importpath")
                            .num_args(1)
                            .value_parser(ValueParser::path_buf())
                            .action(ArgAction::Append),
                    )
                    .arg(
                        Arg::new("IMPORTMAP")
                            .help("Map directory to search for solidity files [format: map=path]")
                            .short('m')
                            .long("importmap")
                            .num_args(1)
                            .value_parser(ValueParser::new(parse_import_map))
                            .action(ArgAction::Append),
                    ),
            )
            .subcommand(
                Command::new("idl")
                    .about("Generate Solidity interface files from Anchor IDL files")
                    .arg(
                        Arg::new("INPUT")
                            .help("Convert IDL files")
                            .required(true)
                            .value_parser(ValueParser::os_string())
                            .num_args(1..),
                    )
                    .arg(
                        Arg::new("OUTPUT")
                            .help("output file")
                            .short('o')
                            .long("output")
                            .num_args(1)
                            .value_parser(ValueParser::os_string()),
                    ),
            )
            .subcommand(
                Command::new("shell-complete")
                    .about("Print shell completion for various shells to STDOUT")
                    .arg(
                        Arg::new("SHELL")
                            .help("Name of a supported shell")
                            .required(true)
                            .value_parser(value_parser!(Shell)),
                    ),
            )
    };
    let matches = app().get_matches();

    match matches.subcommand() {
        Some(("language-server", matches)) => {
            let target = target_arg(matches);

            languageserver::start_server(target, matches);
        }
        Some(("compile", matches)) => compile(matches),
        Some(("doc", matches)) => doc(matches),
        Some(("idl", matches)) => idl::idl(matches),
        Some(("shell-complete", matches)) => shell_complete(app(), matches),
        _ => unreachable!(),
    }
}

fn doc(matches: &ArgMatches) {
    let target = target_arg(matches);
    let mut resolver = imports_arg(matches);

    let verbose = *matches.get_one::<bool>("VERBOSE").unwrap();
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
                .get_one::<OsString>("OUTPUT")
                .unwrap_or(&OsString::from(".")),
            &files,
            verbose,
        );
    }
}

fn compile(matches: &ArgMatches) {
    let target = target_arg(matches);

    let verbose = *matches.get_one::<bool>("VERBOSE").unwrap();
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

    let generate_debug_info = matches.contains_id("GENERATEDEBUGINFORMATION");

    let log_api_return_codes = *matches.get_one::<bool>("LOGAPIRETURNS").unwrap();

    let mut resolver = imports_arg(matches);

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
        generate_debug_information: generate_debug_info,
        common_subexpression_elimination: *matches
            .get_one::<bool>("COMMONSUBEXPRESSIONELIMINATION")
            .unwrap(),
        opt_level,
        log_api_return_codes,
    };

    let mut namespaces = Vec::new();

    let mut errors = false;

    for filename in matches.get_many::<OsString>("INPUT").unwrap() {
        match process_file(filename, &mut resolver, target, matches, &mut json, &opt) {
            Ok(ns) => namespaces.push(ns),
            Err(_) => {
                errors = true;
            }
        }
    }

    if let Some("ast-dot") = matches.get_one::<String>("EMIT").map(|v| v.as_str()) {
        exit(0);
    }

    if errors {
        if matches.contains_id("STD-JSON") {
            println!("{}", serde_json::to_string(&json).unwrap());
            exit(0);
        } else {
            eprintln!("error: not all contracts are valid");
            exit(1);
        }
    }

    if matches.contains_id("STD-JSON") {
        println!("{}", serde_json::to_string(&json).unwrap());
    }
}

fn shell_complete(mut app: Command, matches: &ArgMatches) {
    if let Some(generator) = matches.get_one::<Shell>("SHELL").copied() {
        let name = app.get_name().to_string();
        generate(generator, &mut app, name, &mut std::io::stdout());
    } else {
        eprintln!("Your shell is not supported...");
    }
}

fn output_file(matches: &ArgMatches, stem: &str, ext: &str) -> PathBuf {
    Path::new(
        matches
            .get_one::<OsString>("OUTPUT")
            .unwrap_or(&OsString::from(".")),
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
    let verbose = *matches.get_one::<bool>("VERBOSE").unwrap();

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
            exit(1);
        }

        return Ok(ns);
    }

    if ns.contracts.is_empty() || ns.diagnostics.any_errors() {
        return Err(());
    }

    // emit phase
    for contract_no in 0..ns.contracts.len() {
        let resolved_contract = &ns.contracts[contract_no];

        if !resolved_contract.instantiable {
            continue;
        }

        if let Some("cfg") = matches.get_one::<String>("EMIT").map(|v| v.as_str()) {
            println!("{}", resolved_contract.print_cfg(&ns));
            continue;
        }

        if verbose {
            if target == solang::Target::Solana {
                eprintln!(
                    "info: contract {} uses at least {} bytes account data",
                    resolved_contract.name, resolved_contract.fixed_layout_size,
                );
            }

            eprintln!(
                "info: Generating LLVM IR for contract {} with target {}",
                resolved_contract.name, ns.target
            );
        }

        let context = inkwell::context::Context::create();
        let filename_string = filename.to_string_lossy();

        let binary = resolved_contract.binary(&ns, &context, &filename_string, opt);

        if save_intermediates(&binary, matches) {
            continue;
        }

        let code = binary.code(Generate::Linked).expect("llvm build");

        if matches.contains_id("STD-JSON") {
            json_contracts.insert(
                binary.name.to_owned(),
                JsonContract {
                    abi: abi::ethereum::gen_abi(contract_no, &ns),
                    ewasm: Some(EwasmContract {
                        wasm: hex::encode_upper(code),
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

            file.write_all(&code).unwrap();

            let (abi_bytes, abi_ext) = abi::generate_abi(contract_no, &ns, &code, verbose);
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
    let verbose = *matches.get_one::<bool>("VERBOSE").unwrap();

    match matches.get_one::<String>("EMIT").map(|v| v.as_str()) {
        Some("llvm-ir") => {
            let llvm_filename = output_file(matches, &binary.name, "ll");

            if verbose {
                eprintln!(
                    "info: Saving LLVM IR {} for contract {}",
                    llvm_filename.display(),
                    binary.name
                );
            }

            binary.dump_llvm(&llvm_filename).unwrap();

            true
        }

        Some("llvm-bc") => {
            let bc_filename = output_file(matches, &binary.name, "bc");

            if verbose {
                eprintln!(
                    "info: Saving LLVM BC {} for contract {}",
                    bc_filename.display(),
                    binary.name
                );
            }

            binary.bitcode(&bc_filename);

            true
        }

        Some("object") => {
            let obj = match binary.code(Generate::Object) {
                Ok(o) => o,
                Err(s) => {
                    println!("error: {}", s);
                    exit(1);
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
                    exit(1);
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
            exit(1);
        }
    }

    match File::create(path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("error: cannot create file '{}': {}", path.display(), err,);
            exit(1);
        }
    }
}

fn target_arg(matches: &ArgMatches) -> Target {
    let address_length = matches.get_one::<u64>("ADDRESS_LENGTH").unwrap();

    let value_length = matches.get_one::<u64>("VALUE_LENGTH").unwrap();

    let target = match matches.get_one::<String>("TARGET").unwrap().as_str() {
        "solana" => solang::Target::Solana,
        "substrate" => solang::Target::Substrate {
            address_length: *address_length as usize,
            value_length: *value_length as usize,
        },
        "evm" => solang::Target::EVM,
        _ => unreachable!(),
    };

    if !target.is_substrate()
        && matches.value_source("ADDRESS_LENGTH") == Some(ValueSource::CommandLine)
    {
        eprintln!(
            "error: address length cannot be modified for target '{}'",
            target
        );
        exit(1);
    }

    if !target.is_substrate()
        && matches.value_source("VALUE_LENGTH") == Some(ValueSource::CommandLine)
    {
        eprintln!(
            "error: value length cannot be modified for target '{}'",
            target
        );
        exit(1);
    }

    target
}

fn imports_arg(matches: &ArgMatches) -> FileResolver {
    let mut resolver = FileResolver::new();

    for filename in matches.get_many::<OsString>("INPUT").unwrap() {
        if let Ok(path) = PathBuf::from(filename).canonicalize() {
            let _ = resolver.add_import_path(path.parent().unwrap());
        }
    }

    if let Err(e) = resolver.add_import_path(&PathBuf::from(".")) {
        eprintln!("error: cannot add current directory to import path: {}", e);
        exit(1);
    }

    if let Some(paths) = matches.get_many::<PathBuf>("IMPORTPATH") {
        for path in paths {
            if let Err(e) = resolver.add_import_path(path) {
                eprintln!("error: import path '{}': {}", path.to_string_lossy(), e);
                exit(1);
            }
        }
    }

    if let Some(maps) = matches.get_many::<(String, PathBuf)>("IMPORTMAP") {
        for (map, path) in maps {
            if let Err(e) = resolver.add_import_map(OsString::from(map), path.clone()) {
                eprintln!("error: import path '{}': {}", path.display(), e);
                exit(1);
            }
        }
    }

    resolver
}

// Parse the import map argument. This takes the form
/// --import-map openzeppelin=/opt/openzeppelin-contracts/contract,
/// and returns the name of the map and the path.
fn parse_import_map(map: &str) -> Result<(String, PathBuf), String> {
    if let Some((var, value)) = map.split_once('=') {
        Ok((var.to_owned(), PathBuf::from(value)))
    } else {
        Err("contains no '='".to_owned())
    }
}
