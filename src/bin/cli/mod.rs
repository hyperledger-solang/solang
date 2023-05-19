// SPDX-License-Identifier: Apache-2.0

use clap::{builder::ValueParser, value_parser, ArgAction, Args, Parser, Subcommand};
use clap_complete::Shell;

use std::{ffi::OsString, path::PathBuf, process::exit};

use solang::{
    codegen::{OptimizationLevel, Options},
    file_resolver::FileResolver,
    Target,
};

#[derive(Parser)]
#[command(author = env!("CARGO_PKG_AUTHORS"), version = concat!("version ", env!("SOLANG_VERSION")), about = env!("CARGO_PKG_DESCRIPTION"), subcommand_required = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Compile Solidity source files")]
    Compile(Compile),

    #[command(about = "Generate documention for contracts using doc comments")]
    Doc(Doc),

    #[command(about = "Print shell completion for various shells to STDOUT")]
    ShellComplete(ShellComplete),

    #[command(about = "Start LSP language server on stdin/stdout")]
    LanguageServer(LanguageServerCommand),

    #[command(about = "Generate Solidity interface files from Anchor IDL files")]
    Idl(IdlCommand),
}

#[derive(Args)]
pub struct IdlCommand {
    #[arg(name = "INPUT", help = "Convert IDL files", required= true, value_parser = ValueParser::os_string(), num_args = 1..)]
    pub input: Vec<OsString>,

    #[arg(name = "OUTPUT",help = "output file", short = 'o', long = "output", num_args = 1, value_parser =ValueParser::path_buf())]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct LanguageServerCommand {
    #[clap(flatten)]
    pub target: TargetArg,

    #[arg(name = "IMPORTPATH", help = "Directory to search for solidity files", value_parser = ValueParser::path_buf() , action = ArgAction::Append, long = "importpath", short = 'I', num_args = 1)]
    pub import_path: Option<Vec<PathBuf>>,

    #[arg(name = "IMPORTMAP", help = "Map directory to search for solidity files [format: map=path]",value_parser = ValueParser::new(parse_import_map) , action = ArgAction::Append, long = "importmap", short = 'm', num_args = 1)]
    pub import_map: Option<Vec<(String, PathBuf)>>,
}

#[derive(Args)]
pub struct ShellComplete {
    #[arg(required = true, value_parser = value_parser!(Shell), help = "Name of a supported shell")]
    pub shell_complete: Shell,
}

#[derive(Args)]
pub struct Doc {
    #[clap(flatten)]
    pub package: Package,

    #[clap(flatten)]
    pub target: TargetArg,

    #[arg(name = "VERBOSE" ,help = "show debug messages", short = 'v', action = ArgAction::SetTrue, long = "verbose")]
    pub verbose: bool,

    #[arg(name = "OUTPUT",help = "output directory", short = 'o', long = "output", num_args = 1, value_parser =ValueParser::string())]
    pub output_directory: Option<OsString>,
}

#[derive(Args)]
pub struct Compile {
    #[clap(flatten)]
    pub package: Package,

    #[clap(flatten)]
    pub compiler_output: CompilerOutput,

    #[clap(flatten)]
    pub target_arg: TargetArg,

    #[clap(flatten)]
    pub debug_features: DebugFeatures,

    #[clap(flatten)]
    pub optimizations: Optimizations,
}

#[derive(Args)]
pub struct CompilerOutput {
    #[arg(name = "EMIT", help = "Emit compiler state at early stage", long = "emit", num_args = 1, value_parser = ["ast-dot", "cfg", "llvm-ir", "llvm-bc", "object", "asm"])]
    pub emit: Option<String>,

    #[arg(name = "STD-JSON",help = "mimic solidity json output on stdout", conflicts_with_all = ["VERBOSE", "OUTPUT", "EMIT"] , action = ArgAction::SetTrue, long = "standard-json")]
    pub std_json_output: bool,

    #[arg(name = "OUTPUT",help = "output directory", short = 'o', long = "output", num_args = 1, value_parser =ValueParser::string())]
    pub output_directory: Option<String>,

    #[arg(name = "OUTPUTMETA",help = "output directory for metadata", long = "output-meta", num_args = 1, value_parser = ValueParser::string())]
    pub output_meta: Option<String>,

    #[arg(name = "VERBOSE" ,help = "show debug messages", short = 'v', action = ArgAction::SetTrue, long = "verbose")]
    pub verbose: bool,
}

#[derive(Args)]
pub struct TargetArg {
    #[arg(name = "TARGET",required= true, long = "target", value_parser = ["solana", "substrate", "evm"], help = "Target to build for [possible values: solana, substrate]", num_args = 1, hide_possible_values = true)]
    pub name: String,

    #[arg(name = "ADDRESS_LENGTH", help = "Address length on Substrate", long = "address-length", num_args = 1, value_parser = value_parser!(u64).range(4..1024))]
    pub address_length: Option<u64>,

    #[arg(name = "VALUE_LENGTH", help = "Value length on Substrate", long = "value-length", num_args = 1, value_parser = value_parser!(u64).range(4..1024))]
    pub value_length: Option<u64>,
}

#[derive(Args)]
pub struct Package {
    #[arg(name = "INPUT", help = "Solidity input files", required= true, value_parser = ValueParser::os_string(), num_args = 1..)]
    pub input: Vec<OsString>,

    #[arg(name = "CONTRACT", help = "Contract names to compile (defaults to all)", value_delimiter = ',', action = ArgAction::Append, long = "contract")]
    pub contracts: Option<Vec<String>>,

    #[arg(name = "IMPORTPATH", help = "Directory to search for solidity files",value_parser = ValueParser::path_buf() , action = ArgAction::Append, long = "importpath", short = 'I', num_args = 1)]
    pub import_path: Option<Vec<PathBuf>>,

    #[arg(name = "IMPORTMAP", help = "Map directory to search for solidity files [format: map=path]",value_parser = ValueParser::new(parse_import_map) , action = ArgAction::Append, long = "importmap", short = 'm', num_args = 1)]
    pub import_map: Option<Vec<(String, PathBuf)>>,
}

#[derive(Args)]
pub struct DebugFeatures {
    #[arg(name = "NOLOGAPIRETURNS", help = "Disable logging the return codes of runtime API calls in the environment", long = "no-log-api-return-codes", action = ArgAction::SetFalse)]
    pub log_api_return_codes: bool,

    #[arg(name = "NOLOGRUNTIMEERRORS", help = "Disable logging runtime errors in the environment", long = "no-log-runtime-errors", action = ArgAction::SetFalse)]
    pub log_runtime_errors: bool,

    #[arg(name = "NOPRINTS", help = "Disable logging prints in the environment", long = "no-prints", action = ArgAction::SetFalse)]
    pub log_prints: bool,

    #[arg(name = "GENERATEDEBUGINFORMATION", help = "Enable generating debug information for LLVM IR", long = "generate-debug-info", action = ArgAction::SetTrue, short = 'g')]
    pub generate_debug_info: bool,

    #[arg(name = "RELEASE", help = "Disable all debugging features such as prints, logging runtime errors, and logging api return codes", long = "release", action = ArgAction::SetTrue)]
    pub release: bool,
}

#[derive(Args)]
pub struct Optimizations {
    #[arg(name = "DEADSTORAGE", help = "Disable dead storage codegen optimization", long = "no-dead-storage", action = ArgAction::SetFalse, display_order = 3)]
    pub dead_storage: bool,

    #[arg(name = "CONSTANTFOLDING", help = "Disable constant folding codegen optimization", long = "no-constant-folding", action = ArgAction::SetFalse, display_order = 1)]
    pub constant_folding: bool,

    #[arg(name = "STRENGTHREDUCE", help = "Disable strength reduce codegen optimization", long = "no-strength-reduce", action = ArgAction::SetFalse, display_order = 2)]
    pub strength_reduce: bool,

    #[arg(name = "VECTORTOSLICE", help = "Disable vector to slice codegen optimization", long = "no-vector-to-slice", action = ArgAction::SetFalse, display_order = 4)]
    pub vector_to_slice: bool,

    #[arg(name = "COMMONSUBEXPRESSIONELIMINATION", help = "Disable common subexpression elimination", long = "no-cse", action = ArgAction::SetFalse, display_order = 5)]
    pub common_subexpression_elimination: bool,

    #[arg(name = "OPT", help = "Set llvm optimizer level ", short = 'O', default_value = "default", value_parser = ["none", "less", "default", "aggressive"], num_args = 1)]
    pub opt_level: String,
}

pub(crate) fn target_arg(target_arg: &TargetArg) -> Target {
    if target_arg.name.as_str() == "solana" || target_arg.name.as_str() == "evm" {
        if target_arg.address_length.is_some() {
            eprintln!("error: address length cannot be modified except for substrate target");
            exit(1);
        }

        if target_arg.value_length.is_some() {
            eprintln!("error: value length cannot be modified except for substrate target");
            exit(1);
        }
    }

    let target = match target_arg.name.as_str() {
        "solana" => solang::Target::Solana,
        "substrate" => solang::Target::Substrate {
            address_length: target_arg.address_length.unwrap_or(32) as usize,
            value_length: target_arg.value_length.unwrap_or(16) as usize,
        },
        "evm" => solang::Target::EVM,
        _ => unreachable!(),
    };

    target
}

pub fn imports_arg(package: &Package) -> FileResolver {
    let mut resolver = FileResolver::new();

    for filename in &package.input {
        if let Ok(path) = PathBuf::from(filename).canonicalize() {
            let _ = resolver.add_import_path(path.parent().unwrap());
        }
    }

    if let Err(e) = resolver.add_import_path(&PathBuf::from(".")) {
        eprintln!("error: cannot add current directory to import path: {e}");
        exit(1);
    }

    if let Some(paths) = &package.import_path {
        for path in paths {
            if let Err(e) = resolver.add_import_path(path) {
                eprintln!("error: import path '{}': {}", path.to_string_lossy(), e);
                exit(1);
            }
        }
    }

    if let Some(maps) = &package.import_map {
        for (map, path) in maps {
            if let Err(e) = resolver.add_import_map(OsString::from(map), path.clone()) {
                eprintln!("error: import path '{}': {}", path.display(), e);
                exit(1);
            }
        }
    }

    resolver
}

pub fn options_arg(debug: &DebugFeatures, optimizations: &Optimizations) -> Options {
    let opt_level = match optimizations.opt_level.as_str() {
        "none" => OptimizationLevel::None,
        "less" => OptimizationLevel::Less,
        "default" => OptimizationLevel::Default,
        "aggressive" => OptimizationLevel::Aggressive,
        _ => unreachable!(),
    };
    Options {
        dead_storage: optimizations.dead_storage,
        constant_folding: optimizations.constant_folding,
        strength_reduce: optimizations.strength_reduce,
        vector_to_slice: optimizations.vector_to_slice,
        common_subexpression_elimination: optimizations.common_subexpression_elimination,
        generate_debug_information: debug.generate_debug_info,
        opt_level,
        log_api_return_codes: debug.log_api_return_codes,
        log_runtime_errors: debug.log_runtime_errors,
        log_prints: debug.log_prints,
    }
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
