// SPDX-License-Identifier: Apache-2.0

use clap::{Command, CommandFactory, Parser};

use clap_complete::generate;
use itertools::Itertools;
use solang::{
    abi,
    codegen::{codegen, Options},
    emit::Generate,
    file_resolver::FileResolver,
    sema::{ast::Namespace, file::PathDisplay},
    standard_json::{EwasmContract, JsonContract, JsonResult},
};
use std::{
    collections::{HashMap, HashSet},
    ffi::{OsStr, OsString},
    fs::{create_dir_all, File},
    io::prelude::*,
    path::{Path, PathBuf},
    process::exit,
};

use crate::cli::{
    imports_arg, options_arg, target_arg, Cli, Commands, Compile, CompilerOutput, Doc,
    ShellComplete,
};

mod cli;
mod doc;
mod idl;
mod languageserver;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doc(doc_args) => doc(doc_args),
        Commands::Compile(compile_args) => compile(&compile_args),
        Commands::ShellComplete(shell_args) => shell_complete(Cli::command(), shell_args),
        Commands::LanguageServer(server_args) => languageserver::start_server(&server_args),
        Commands::Idl(idl_args) => idl::idl(&idl_args),
    }
}

fn doc(doc_args: Doc) {
    let target = target_arg(&doc_args.target);
    let mut resolver = imports_arg(&doc_args.package);

    let verbose = doc_args.verbose;
    let mut success = true;
    let mut files = Vec::new();

    for filename in doc_args.package.input {
        let ns = solang::parse_and_resolve(&filename, &mut resolver, target);

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
            &doc_args
                .output_directory
                .unwrap_or_else(|| OsString::from(".")),
            &files,
            verbose,
        );
    }
}

fn compile(compile_args: &Compile) {
    let mut json = JsonResult {
        errors: Vec::new(),
        target: compile_args.target_arg.name.clone(),
        program: String::new(),
        contracts: HashMap::new(),
    };

    if compile_args.compiler_output.verbose {
        eprintln!("info: Solang version {}", env!("SOLANG_VERSION"));
    }

    let mut resolver = imports_arg(&compile_args.package);

    let opt = options_arg(&compile_args.debug_features, &compile_args.optimizations);

    let target = target_arg(&compile_args.target_arg);

    let mut namespaces = Vec::new();

    let mut errors = false;

    // Build a map of requested contract names, and a flag specifying whether it was found or not
    let contract_names: HashSet<&str> = if let Some(values) = &compile_args.package.contracts {
        values.iter().map(String::as_str).collect()
    } else {
        HashSet::new()
    };

    for filename in &compile_args.package.input {
        // TODO: this could be parallelized using e.g. rayon
        let ns = process_file(
            filename,
            &mut resolver,
            target,
            &compile_args.compiler_output,
            &opt,
        );

        namespaces.push(ns);
    }

    let mut json_contracts = HashMap::new();

    let std_json = compile_args.compiler_output.std_json_output;

    for ns in &namespaces {
        if std_json {
            let mut out = ns.diagnostics_as_json(&resolver);
            json.errors.append(&mut out);
        } else {
            ns.print_diagnostics(&resolver, compile_args.compiler_output.verbose);
        }

        if ns.diagnostics.any_errors() {
            errors = true;
        }
    }

    if let Some("ast-dot") = compile_args.compiler_output.emit.as_deref() {
        exit(0);
    }

    // Ensure we have at least one contract
    if !errors && namespaces.iter().all(|ns| ns.contracts.is_empty()) {
        eprintln!("error: no contacts found");
        errors = true;
    }

    // Ensure we have all the requested contracts
    let not_found: Vec<_> = contract_names
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
                    &compile_args.compiler_output,
                    ns,
                    &mut json_contracts,
                    &mut seen_contracts,
                    &opt,
                );
            }
        }
    }

    if std_json {
        println!("{}", serde_json::to_string(&json).unwrap());
        exit(0);
    }

    if errors {
        exit(1);
    }
}

fn shell_complete(mut app: Command, args: ShellComplete) {
    let name = app.get_name().to_string();
    generate(args.shell_complete, &mut app, name, &mut std::io::stdout());
}

fn output_file(compiler_output: &CompilerOutput, stem: &str, ext: &str, meta: bool) -> PathBuf {
    let dir = if meta {
        compiler_output
            .output_meta
            .as_ref()
            .or(compiler_output.output_directory.as_ref())
    } else {
        compiler_output.output_directory.as_ref()
    };
    Path::new(&dir.unwrap_or(&String::from("."))).join(format!("{stem}.{ext}"))
}

fn process_file(
    filename: &OsStr,
    resolver: &mut FileResolver,
    target: solang::Target,
    compiler_output: &CompilerOutput,
    opt: &Options,
) -> Namespace {
    let verbose = compiler_output.verbose;

    // resolve phase
    let mut ns = solang::parse_and_resolve(filename, resolver, target);

    // codegen all the contracts; some additional errors/warnings will be detected here
    codegen(&mut ns, opt);

    if let Some("ast-dot") = compiler_output.emit.as_deref() {
        let filepath = PathBuf::from(filename);
        let stem = filepath.file_stem().unwrap().to_string_lossy();
        let dot_filename = output_file(compiler_output, &stem, "dot", false);

        if verbose {
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
    compiler_output: &CompilerOutput,
    ns: &mut Namespace,
    json_contracts: &mut HashMap<String, JsonContract>,
    seen_contracts: &mut HashMap<String, String>,
    opt: &Options,
) {
    let verbose = compiler_output.verbose;
    let std_json = compiler_output.std_json_output;

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

    if let Some("cfg") = compiler_output.emit.as_deref() {
        println!("{}", resolved_contract.print_cfg(ns));
        return;
    }

    if verbose {
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

    let binary = resolved_contract.binary(ns, &context, opt);

    if save_intermediates(&binary, compiler_output) {
        return;
    }

    let code = binary.code(Generate::Linked).expect("llvm build");

    if std_json {
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
        let bin_filename = output_file(
            compiler_output,
            &binary.name,
            ns.target.file_extension(),
            false,
        );

        if verbose {
            eprintln!(
                "info: Saving binary {} for contract {}",
                bin_filename.display(),
                binary.name
            );
        }

        let mut file = create_file(&bin_filename);

        file.write_all(&code).unwrap();

        let (metadata, meta_ext) = abi::generate_abi(contract_no, ns, &code, verbose);
        let meta_filename = output_file(compiler_output, &binary.name, meta_ext, true);

        if verbose {
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

fn save_intermediates(
    binary: &solang::emit::binary::Binary,
    compiler_output: &CompilerOutput,
) -> bool {
    let verbose = compiler_output.verbose;

    match compiler_output.emit.as_deref() {
        Some("llvm-ir") => {
            let llvm_filename = output_file(compiler_output, &binary.name, "ll", false);

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
            let bc_filename = output_file(compiler_output, &binary.name, "bc", false);

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
                    println!("error: {s}");
                    exit(1);
                }
            };

            let obj_filename = output_file(compiler_output, &binary.name, "o", false);

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
                    println!("error: {s}");
                    exit(1);
                }
            };

            let obj_filename = output_file(compiler_output, &binary.name, "asm", false);

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
