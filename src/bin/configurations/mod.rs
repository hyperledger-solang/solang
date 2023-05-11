// SPDX-License-Identifier: Apache-2.0

use clap::{parser::ValueSource, ArgMatches};
use solang::{
    codegen::{OptimizationLevel, Options},
    file_resolver::FileResolver,
    Target,
};

use std::{collections::HashSet, ffi::OsString, fs, path::PathBuf, process::exit};
use toml::Table;

// A macro for error handling.
macro_rules! match_err {
    ($func_name: expr) => {
        match $func_name {
            Ok(value) => value,
            Err(err) => {
                return Err(format!("{err}"));
            }
        }
    };
}

// A type that defines the package to be compiled: file names, contract names, import maps and import paths.
type Package = (
    Vec<OsString>,
    HashSet<String>,
    Vec<PathBuf>,
    Vec<(String, PathBuf)>,
);

type CompilerOutput = (
    bool,
    bool,
    Option<String>,
    Option<OsString>,
    Option<OsString>,
);

// All compile subcommand configurations
pub struct Configurations {
    pub options: Options,
    pub verbose: bool,
    pub target: Target,
    pub filenames: Vec<OsString>,
    pub contract_names: HashSet<String>,
    pub imports: FileResolver,
    pub emit: Option<String>,
    pub std_json_output: bool,
    pub output_directory: Option<OsString>,
    pub output_meta: Option<OsString>,
}

impl Configurations {
    /// Parse compile configurations from a toml file.
    pub fn config_from_toml(path: &PathBuf) -> Result<Self, String> {
        let toml_data = match_err!(fs::read_to_string(path).or(Err("couldn't read toml file")));
        let cfg: Table = match_err!(toml_data.parse());
        let target = match_err!(Self::parse_target(&cfg));
        let (filenames, contract_names, _, _) = match_err!(Self::parse_package(&cfg));
        let imports = match_err!(Self::imports_arg(None, Some(&cfg)));
        let options = match_err!(Self::parse_options(&cfg));
        let (verbose, std_json_output, emit, output_directory, output_meta) =
            match_err!(Self::parse_compiler_output(&cfg));

        Ok(Configurations {
            options,
            verbose,
            target,
            filenames,
            contract_names,
            imports,
            emit,
            std_json_output,
            output_directory,
            output_meta,
        })
    }

    /// Parse compiler-output field from the configuration toml.
    fn parse_compiler_output(cfg: &Table) -> Result<CompilerOutput, String> {
        // Providing defaults in case there is no provided values
        let mut verbose = false;
        let mut std_json_output = false;
        let mut emit: Option<String> = None;
        let mut output_directory: Option<OsString> = None;
        let mut output_meta: Option<OsString> = None;

        if let Some(table) = cfg.get("compiler-output") {
            let compiler_out = match_err!(Table::try_from(table));

            for key in compiler_out.keys() {
                match key.as_str() {
                    "verbose" => {
                        verbose = match_err!(Self::check_boolean_value(&compiler_out, key))
                    }
                    "std_json_output" => {
                        std_json_output = match_err!(Self::check_boolean_value(&compiler_out, key))
                    }
                    "emit" => match table.get(key).unwrap().as_str().unwrap() {
                        "ast-dot" | "cfg" | "llvm-ir" | "llvm-bc" | "object" | "asm" => {
                            emit = Some(String::from(table.get(key).unwrap().as_str().unwrap()))
                        }

                        _ => return Err("Unrecognized option for `emit`.".to_owned()),
                    },

                    "output_directory" => {
                        output_directory =
                            Some(OsString::from(table.get(key).unwrap().as_str().unwrap()))
                    }
                    "metadata_output_directory" => {
                        output_meta =
                            Some(OsString::from(table.get(key).unwrap().as_str().unwrap()))
                    }
                    _ => return Err(format!("Invalid option {}", key.as_str())),
                }
            }
        };

        Ok((
            verbose,
            std_json_output,
            emit,
            output_directory,
            output_meta,
        ))
    }

    fn parse_package(cfg: &Table) -> Result<Package, String> {
        let package_table = match cfg
            .get("package")
            .ok_or("No package field defined in toml file")
        {
            Ok(value) => match_err!(Table::try_from(value)),
            Err(err) => {
                return Err(err.to_owned());
            }
        };

        let filenames = match package_table
            .get("input_files")
            .ok_or("No input_files defined in .toml file")
        {
            Ok(value) => {
                match_err!(value.as_array().ok_or(
                "Wrong argument for key `input_files`. please define as `input_files = [`file1`, `file2`, ...]`"
            ))
            }

            Err(err) => {
                return Err(err.to_owned());
            }
        }
        .iter()
        .map(|x| OsString::from(x.as_str().unwrap()))
        .collect();

        let contract_names: HashSet<String> = if let Some(values) = package_table.get("contracts") {
            match_err!(values.as_array().ok_or("Wrong argument for key `contracts`. please define as `contracts = [`contract1`, `contract2`, ...]`"))
                .iter()
                .map(|x| String::from(x.as_str().unwrap()))
                .collect()
        } else {
            HashSet::new()
        };

        let import_paths: Vec<PathBuf> = if let Some(values) = package_table.get("import_paths") {
            match_err!(values.as_array().ok_or("Wrong argument for key `import_paths`. please define as `import_paths = [`path1`, `path2`, ...]`"))

                .iter()
                .map(|x| PathBuf::from(x.as_str().unwrap()))
                .collect()
        } else {
            Vec::new()
        };

        let import_maps = if let Some(values) = package_table.get("import_maps") {
            let result_vec : Result<Vec<(String, PathBuf)>, String> = match_err!( values
                .as_array()
                .ok_or(
                    "Wrong argument for key `import_maps`. please define as `import_maps = [`import=path`, `import=path`, ..]`",
                ))
                .iter().map(|x| Self::parse_import_map(x.as_str().unwrap()) ).collect();
            match_err!(result_vec)
        } else {
            Vec::new()
        };
        Ok((filenames, contract_names, import_paths, import_maps))
    }

    fn parse_target(cfg: &Table) -> Result<Target, String> {
        let target_table = match_err!(Table::try_from(match_err!(cfg
            .get("target")
            .ok_or("no target field defined in toml file. please specify target."))));

        let target_name = match_err!(target_table.get("name").ok_or(
            "no `name` field defined for `target`. provide either `solana` or `substrate`."
        ))
        .as_str()
        .unwrap();

        match target_name {
            "solana" => Ok(Target::Solana),
            "substrate" => {
                let address_length = match_err!(target_table
                    .get("address_length")
                    .unwrap_or(&toml::Value::Integer(32))
                    .as_integer()
                    .ok_or("Invalid value for key `address_length`"));

                let value_length = match_err!(target_table
                    .get("value_length")
                    .unwrap_or(&toml::Value::Integer(16))
                    .as_integer()
                    .ok_or("Invalid value for key `value_length`"));

                Ok(Target::Substrate {
                    address_length: address_length as usize,
                    value_length: value_length as usize,
                })
            }
            "evm" => Ok(Target::EVM),

            _ => Err("provided invalid target".to_owned()),
        }
    }

    fn parse_options(cfg: &Table) -> Result<Options, String> {
        let mut options = Options::default();

        if let Some(table) = cfg.get("optimizations") {
            let opt_table = match_err!(Table::try_from(table));

            for key in opt_table.keys() {
                match key.as_str() {
                    "dead_storage" => {
                        options.dead_storage =
                            match_err!(Self::check_boolean_value(&opt_table, key))
                    }
                    "constant_folding" => {
                        options.constant_folding =
                            match_err!(Self::check_boolean_value(&opt_table, key))
                    }
                    "strength_reduce" => {
                        options.strength_reduce =
                            match_err!(Self::check_boolean_value(&opt_table, key))
                    }
                    "vector_to_slice" => {
                        options.vector_to_slice =
                            match_err!(Self::check_boolean_value(&opt_table, key))
                    }
                    "common_subexpression_elimination" => {
                        options.common_subexpression_elimination =
                            match_err!(Self::check_boolean_value(&opt_table, key))
                    }
                    "llvm-IR_optimization_level" => {
                        let opt_level = match opt_table.get(key).unwrap().as_str().unwrap() {
                            "none" => OptimizationLevel::None,
                            "less" => OptimizationLevel::Less,
                            "default" => OptimizationLevel::Default,
                            "aggressive" => OptimizationLevel::Aggressive,
                            _ => {
                                eprintln!(
                                "Invalid option for optimization level, going for default option"
                            );
                                OptimizationLevel::Default
                            }
                        };
                        options.opt_level = opt_level;
                    }
                    _ => {
                        return Err(format!("unrecognized option {key}."));
                    }
                }
            }
        } else {
            eprintln!("No debug features defined in toml file, going for default values")
        };

        if let Some(table) = cfg.get("debug-features") {
            let debug_table = match_err!(Table::try_from(table));

            for key in debug_table.keys() {
                match key.as_str() {
                    "log-api-return-codes" => {
                        options.log_api_return_codes =
                            match_err!(Self::check_boolean_value(&debug_table, key))
                    }
                    "log-runtime-errors" => {
                        options.log_runtime_errors =
                            match_err!(Self::check_boolean_value(&debug_table, key))
                    }
                    "prints" => {
                        options.log_prints =
                            match_err!(Self::check_boolean_value(&debug_table, key))
                    }
                    "llvm-IR-debug-info" => {
                        options.generate_debug_information =
                            match_err!(Self::check_boolean_value(&debug_table, key))
                    }

                    _ => return Err(format!("unrecognized option {key}.")),
                }
            }
        }

        Ok(options)
    }

    fn check_boolean_value(cfg: &Table, key: &String) -> Result<bool, String> {
        match cfg
            .get(key)
            .unwrap()
            .as_bool()
            .ok_or(format!("Enter `true` or `false` for the key {key}"))
        {
            Ok(value) => Ok(value),
            Err(err) => Err(err),
        }
    }

    pub fn config_from_matches(matches: &ArgMatches) -> Self {
        let filenames = Self::filenames_from_matches(matches);
        Configurations {
            options: Options {
                dead_storage: *matches.get_one("DEADSTORAGE").unwrap(),
                constant_folding: *matches.get_one("CONSTANTFOLDING").unwrap(),
                strength_reduce: *matches.get_one("STRENGTHREDUCE").unwrap(),
                vector_to_slice: *matches.get_one("VECTORTOSLICE").unwrap(),
                generate_debug_information: *matches.get_one("GENERATEDEBUGINFORMATION").unwrap(),
                common_subexpression_elimination: *matches
                    .get_one("COMMONSUBEXPRESSIONELIMINATION")
                    .unwrap(),
                opt_level: match matches.get_one::<String>("OPT").unwrap().as_str() {
                    "none" => OptimizationLevel::None,
                    "less" => OptimizationLevel::Less,
                    "default" => OptimizationLevel::Default,
                    "aggressive" => OptimizationLevel::Aggressive,
                    _ => unreachable!(),
                },
                log_api_return_codes: *matches.get_one("NOLOGAPIRETURNS").unwrap(),
                log_runtime_errors: *matches.get_one::<bool>("NOLOGRUNTIMEERRORS").unwrap(),
                log_prints: *matches.get_one::<bool>("NOPRINT").unwrap(),
            },

            verbose: *matches.get_one::<bool>("VERBOSE").unwrap(),

            target: Self::target_arg(matches),

            filenames,

            // Build a map of requested contract names, and a flag specifying whether it was found or not
            contract_names: if let Some(names) = matches.get_many::<String>("CONTRACT") {
                names.cloned().collect()
            } else {
                HashSet::new()
            },

            imports: Self::imports_arg(Some(matches), None).unwrap(),
            emit: matches.get_one::<String>("EMIT").cloned(),
            std_json_output: *matches.get_one("STD-JSON").unwrap(),
            output_directory: matches.get_one::<OsString>("OUTPUT").cloned(),
            output_meta: matches.get_one::<OsString>("OUTPUTMETA").cloned(),
        }
    }

    fn filenames_from_matches(matches: &ArgMatches) -> Vec<OsString> {
        matches
            .get_many::<OsString>("INPUT")
            .unwrap()
            .cloned()
            .collect()
    }

    pub fn imports_arg(
        matches: Option<&ArgMatches>,
        conf_file: Option<&Table>,
    ) -> Result<FileResolver, String> {
        let (filenames, import_path, import_map) = if let Some(path) = conf_file {
            let files = match_err!(Self::parse_package(path));

            (files.0, files.2, files.3)
        } else {
            let filenames: Vec<OsString> = matches
                .unwrap()
                .get_many::<OsString>("INPUT")
                .unwrap()
                .cloned()
                .collect();

            let import_path =
                if let Some(paths) = matches.unwrap().get_many::<PathBuf>("IMPORTPATH") {
                    paths.cloned().collect()
                } else {
                    Vec::new()
                };

            let import_map =
                if let Some(maps) = matches.unwrap().get_many::<(String, PathBuf)>("IMPORTMAP") {
                    maps.cloned().collect()
                } else {
                    Vec::new()
                };

            (filenames, import_path, import_map)
        };

        let mut resolver = FileResolver::new();

        for filename in filenames {
            if let Ok(path) = PathBuf::from(filename).canonicalize() {
                let _ = resolver.add_import_path(path.parent().unwrap());
            }
        }

        if let Err(e) = resolver.add_import_path(&PathBuf::from(".")) {
            eprintln!("error: cannot add current directory to import path: {e}");
            exit(1);
        }

        for path in import_path {
            if let Err(e) = resolver.add_import_path(&path) {
                eprintln!("error: import path '{}': {}", path.to_string_lossy(), e);
                exit(1);
            }
        }

        for (map, path) in import_map {
            if let Err(e) = resolver.add_import_map(OsString::from(map), path.clone()) {
                eprintln!("error: import path '{}': {}", path.display(), e);
                exit(1);
            }
        }

        Ok(resolver)
    }

    pub fn target_arg(matches: &ArgMatches) -> Target {
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
            eprintln!("error: address length cannot be modified for target '{target}'");
            exit(1);
        }

        if !target.is_substrate()
            && matches.value_source("VALUE_LENGTH") == Some(ValueSource::CommandLine)
        {
            eprintln!("error: value length cannot be modified for target '{target}'");
            exit(1);
        }

        target
    }

    /// Parse the import map argument. This takes the fo rm
    /// --import-map openzeppelin=/opt/openzeppelin-contracts/contract,
    /// and returns the name of the map and the path.
    pub fn parse_import_map(map: &str) -> Result<(String, PathBuf), String> {
        if let Some((var, value)) = map.split_once('=') {
            Ok((var.to_owned(), PathBuf::from(value)))
        } else {
            Err("contains no '='".to_owned())
        }
    }
}
