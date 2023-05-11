// SPDX-License-Identifier: Apache-2.0

use clap::{
    builder::{ArgAction, ValueParser},
    value_parser, Arg, ArgMatches, Command,
};
use clap_complete::{generate, Shell};
use itertools::Itertools;
use solang::{
    abi,
    codegen::codegen,
    emit::Generate,
    sema::{ast::Namespace, file::PathDisplay},
    standard_json::{EwasmContract, JsonContract, JsonResult},
};

use std::{
    collections::HashMap,
    env,
    ffi::{OsStr, OsString},
    fs::{create_dir, create_dir_all, File},
    io::prelude::*,
    path::{Path, PathBuf},
    process::exit,
};

#[macro_use]
mod configurations;
mod doc;
mod idl;
mod languageserver;
use crate::configurations::Configurations;

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
                            .value_parser(ValueParser::os_string())
                            .num_args(1..)
                            .required_unless_present("CONF-FILE")
                    ).arg(
                        Arg::new("CONF-FILE")
                            .help("Take arguments from configuration file")
                            .long("configuration-file")
                            .num_args(0..=1)
                            .exclusive(true)
                            .value_parser(ValueParser::os_string())
                            .default_missing_value_os("Solang.toml")
                            .required_unless_present("INPUT")
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
                        Arg::new("CONTRACT")
                            .help("Contract names to compile (defaults to all)")
                            .value_delimiter(',')
                            .action(ArgAction::Append)
                            .long("contract"),
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
                            .help("Target to build for [possible values: solana, substrate]")
                            .long("target")
                            .num_args(1)
                            .value_parser(["solana", "substrate", "evm"])
                            .hide_possible_values(true)
                            .required_unless_present("CONF-FILE"),
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
                            .action(ArgAction::SetTrue)
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
                        Arg::new("OUTPUTMETA")
                            .help("output directory for metadata")
                            .long("output-meta")
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
                            .value_parser(ValueParser::new(Configurations::parse_import_map))
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
                        Arg::new("NOLOGAPIRETURNS")
                            .help("Disable logging the return codes of runtime API calls in the environment")
                            .long("no-log-api-return-codes")
                            .action(ArgAction::SetFalse)
                    )
                    .arg(
                        Arg::new("GENERATEDEBUGINFORMATION")
                            .help("Enable generating debug information for LLVM IR")
                            .short('g')
                            .long("generate-debug-info")
                            .action(ArgAction::SetTrue)
                            .hide(true),
                    )
                    .arg(
                        Arg::new("NOLOGRUNTIMEERRORS")
                            .help("Disable logging runtime errors in the environment")
                            .long("no-log-runtime-errors")
                            .action(ArgAction::SetFalse),
                    ).arg(
                        Arg::new("NOPRINT")
                            .help("Disable logging prints in the environment")
                            .long("no-print")
                            .action(ArgAction::SetFalse),
                    ).arg(
                        Arg::new("RELEASE")
                            .help("Disable all debugging features such as prints, logging runtime errors, and logging api return codes")
                            .long("release")
                            .action(ArgAction::SetTrue),
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
                            .value_parser(ValueParser::new(Configurations::parse_import_map))
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
                            .value_parser(ValueParser::new(Configurations::parse_import_map))
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
            ).subcommand(
                Command::new("new")
                    .about("Creates a new project with default configuations")
                    .arg(
                        Arg::new("TARGET")
                            .help("Target to build for [possible values: solana, substrate]")
                            .long("target")
                            .num_args(1)
                            .value_parser(["solana", "substrate"])
                            .hide_possible_values(true)
                            .required(true),
                    ).arg(
                        Arg::new("INPUT")
                            .help("project name")
                            .value_parser(ValueParser::os_string())
                            .num_args(1),
                    ),
            )
    };

    let matches = app().get_matches();

    match matches.subcommand() {
        Some(("language-server", matches)) => {
            let target = Configurations::target_arg(matches);

            languageserver::start_server(target, matches);
        }
        Some(("compile", matches)) => compile(matches),
        Some(("doc", matches)) => doc(matches),
        Some(("idl", matches)) => idl::idl(matches),
        Some(("shell-complete", matches)) => shell_complete(app(), matches),
        Some(("new", matches)) => solang_new(matches),
        _ => unreachable!(),
    }
}

fn solang_new(matches: &ArgMatches) {
    let target = matches.get_one::<String>("TARGET").unwrap().as_str();

    // Default project name is "solana_project" or "substrate_project"
    let default_path = &OsString::from(format!("{target}_project"));

    let dir_path = matches.get_one::<OsString>("INPUT").unwrap_or(default_path);

    match create_dir(dir_path) {
        Ok(_) => (),
        Err(error) => {
            eprintln!("couldn't create project directory, reason: {error}");
            exit(1)
        }
    };

    let flipper = match target {
        "solana" => include_str!("../../examples/solana/flipper.sol"),
        "substrate" => include_str!("../../examples/substrate/flipper.sol"),
        _ => unreachable!(),
    };

    let mut flipper_file = create_file(&Path::new(dir_path).join("flipper.sol"));
    flipper_file
        .write_all(flipper.to_string().as_bytes())
        .expect("failed to write flipper example");

    let mut toml_file = create_file(&Path::new(dir_path).join("Solang.toml"));

    let toml_content = match target {
        "solana" => include_str!("../../examples/solana/solana.toml"),
        "substrate" => include_str!("../../examples/substrate/substrate.toml"),
        _ => unreachable!(),
    };
    toml_file
        .write_all(toml_content.to_string().as_bytes())
        .expect("failed to write example toml configuration file");
}

fn doc(matches: &ArgMatches) {
    let target = Configurations::target_arg(matches);
    let mut resolver = Configurations::imports_arg(Some(matches), None);

    let verbose = *matches.get_one("VERBOSE").unwrap();
    let mut success = true;
    let mut files = Vec::new();

    for filename in matches.get_many::<&OsString>("INPUT").unwrap() {
        let ns = solang::parse_and_resolve(filename, resolver.as_mut().unwrap(), target);

        ns.print_diagnostics(resolver.as_ref().unwrap(), verbose);

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
    let conf_file = matches.contains_id("CONF-FILE");

    let mut configurations = if conf_file {
        let path = matches.get_one::<OsString>("CONF-FILE").unwrap();

        match Configurations::config_from_toml(&PathBuf::from(path)) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("{err}");
                exit(1);
            }
        }
    } else {
        Configurations::config_from_matches(matches)
    };

    let mut json = JsonResult {
        errors: Vec::new(),
        target: configurations.target.to_string(),
        program: String::new(),
        contracts: HashMap::new(),
    };

    if configurations.verbose {
        eprintln!("info: Solang version {}", env!("SOLANG_VERSION"));
    }

    let mut namespaces = Vec::new();

    let mut errors = false;

    for filename in configurations.filenames.clone() {
        let ns = process_file(&filename, &mut configurations);
        namespaces.push(ns)
    }

    let mut json_contracts = HashMap::new();

    for ns in &namespaces {
        if configurations.std_json_output {
            let mut out = ns.diagnostics_as_json(&configurations.imports);
            json.errors.append(&mut out);
        } else {
            ns.print_diagnostics(&configurations.imports, configurations.verbose);
        }

        if ns.diagnostics.any_errors() {
            errors = true;
        }
    }

    if let Some("ast-dot") = configurations.emit.as_deref() {
        exit(0);
    }

    // Ensure we have at least one contract
    if !errors && namespaces.iter().all(|ns| ns.contracts.is_empty()) {
        eprintln!("error: no contracts found");
        errors = true;
    }

    // Ensure we have all the requested contracts
    let not_found: Vec<_> = configurations
        .contract_names
        .iter()
        .filter(|name| {
            !namespaces
                .iter()
                .flat_map(|ns| ns.contracts.iter())
                .any(|contract| **name == contract.name)
        })
        .collect();

    if !errors && !not_found.is_empty() {
        eprintln!("error: contacts {} not found", not_found.iter().join(", "));
        errors = true;
    }

    if !errors {
        let mut seen_contracts = HashMap::new();

        for ns in namespaces.iter_mut() {
            for contract_no in 0..ns.contracts.len() {
                contract_results(
                    contract_no,
                    &configurations,
                    ns,
                    &mut json_contracts,
                    &mut seen_contracts,
                );
            }
        }
    }

    if configurations.std_json_output {
        println!("{}", serde_json::to_string(&json).unwrap());
        exit(0);
    }

    if errors {
        exit(1);
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

fn output_file(config: &Configurations, stem: &str, ext: &str, meta: bool) -> PathBuf {
    let dir = if meta {
        config
            .output_meta
            .as_deref()
            .or(config.output_directory.as_deref())
    } else {
        config.output_directory.as_deref()
    };

    Path::new(&dir.unwrap_or(&OsString::from("."))).join(format!("{stem}.{ext}"))
}

fn process_file(filename: &OsStr, config: &mut Configurations) -> Namespace {
    // resolve phase
    let mut ns = solang::parse_and_resolve(filename, &mut config.imports, config.target);

    // codegen all the contracts; some additional errors/warnings will be detected here
    codegen(&mut ns, &config.options);

    if let Some("ast-dot") = config.emit.as_deref() {
        let filepath = PathBuf::from(filename);
        let stem = filepath.file_stem().unwrap().to_string_lossy();
        let dot_filename = output_file(config, &stem, "dot", false);

        if config.verbose {
            eprintln!("info: Saving graphviz dot {}", dot_filename.display());
        }

        let dot = ns.dotgraphviz();

        let mut file = create_file(&dot_filename);

        if let Err(err) = file.write_all(dot.as_bytes()) {
            eprintln!("{}: error: {}", dot_filename.display(), err);
            exit(1);
        }
    }

    ns
}

fn contract_results(
    contract_no: usize,
    config: &Configurations,
    ns: &mut Namespace,
    json_contracts: &mut HashMap<String, JsonContract>,
    seen_contracts: &mut HashMap<String, String>,
) {
    let resolved_contract = &ns.contracts[contract_no];

    if !resolved_contract.instantiable {
        return;
    }

    if ns.top_file_no() != resolved_contract.loc.file_no() {
        // contracts that were imported should not be considered. For example, if we have a file
        // a.sol which imports b.sol, and b.sol defines contract B, then:
        // solang compile a.sol
        // should not write the results for contract B
        return;
    }

    let loc = ns.loc_to_string(PathDisplay::FullPath, &resolved_contract.loc);

    if let Some(other_loc) = seen_contracts.get(&resolved_contract.name) {
        eprintln!(
            "error: contract {} defined at {other_loc} and {}",
            resolved_contract.name, loc
        );
        exit(1);
    }

    seen_contracts.insert(resolved_contract.name.to_string(), loc);

    if let Some("cfg") = config.emit.as_deref() {
        println!("{}", resolved_contract.print_cfg(ns));
        return;
    }

    if config.verbose {
        if ns.target == solang::Target::Solana {
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

    let binary = resolved_contract.binary(ns, &context, &config.options);

    if save_intermediates(&binary, config) {
        return;
    }

    let code = binary.code(Generate::Linked).expect("llvm build");

    if config.std_json_output {
        json_contracts.insert(
            binary.name,
            JsonContract {
                abi: abi::ethereum::gen_abi(contract_no, ns),
                ewasm: Some(EwasmContract {
                    wasm: hex::encode_upper(code),
                }),
                minimum_space: None,
            },
        );
    } else {
        let bin_filename = output_file(config, &binary.name, ns.target.file_extension(), false);

        if config.verbose {
            eprintln!(
                "info: Saving binary {} for contract {}",
                bin_filename.display(),
                binary.name
            );
        }

        let mut file = create_file(&bin_filename);

        file.write_all(&code).unwrap();

        let (metadata, meta_ext) = abi::generate_abi(contract_no, ns, &code, config.verbose);
        let meta_filename = output_file(config, &binary.name, meta_ext, true);

        if config.verbose {
            eprintln!(
                "info: Saving metadata {} for contract {}",
                meta_filename.display(),
                binary.name
            );
        }

        let mut file = create_file(&meta_filename);
        file.write_all(metadata.as_bytes()).unwrap();
    }
}

fn save_intermediates(binary: &solang::emit::binary::Binary, config: &Configurations) -> bool {
    match config.emit.as_deref() {
        Some("llvm-ir") => {
            let llvm_filename = output_file(config, &binary.name, "ll", false);

            if config.verbose {
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
            let bc_filename = output_file(config, &binary.name, "bc", false);

            if config.verbose {
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
                    println!("error: {s}");
                    exit(1);
                }
            };

            let obj_filename = output_file(config, &binary.name, "o", false);

            if config.verbose {
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
                    println!("error: {s}");
                    exit(1);
                }
            };

            let obj_filename = output_file(config, &binary.name, "asm", false);

            if config.verbose {
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
