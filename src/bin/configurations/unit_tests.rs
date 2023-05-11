// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
use solang::{
    codegen:: Options,
    Target,
};

use std::{ ffi::OsString, path::PathBuf,str::FromStr};
use toml::Table;

use crate::Configurations;

#[test]
fn compiler_output_test() {
    let compiler_output = r#"[compiler-output]
    verbose = true
    std_json_output = false
    emit = "ast-dot"
    output_directory = "output"
    metadata_output_directory = "metadata"
"#;

    let mut cfg = Table::from_str(compiler_output).unwrap();

    let config = Configurations::parse_compiler_output(&cfg).unwrap();
    assert!(config.0);
    assert!(!config.1);
    assert_eq!(config.2, Some(String::from("ast-dot")));
    assert_eq!(config.3, Some(OsString::from("output")));
    assert_eq!(config.4, Some(OsString::from("metadata")));

    // test each individual case
    cfg = Table::from_str(
        r#"[compiler-output]
    verbose = 1"#,
    )
    .unwrap();

    Configurations::parse_compiler_output(&cfg)
        .expect_err("Enter `true` or `false` for the key verbose");

    cfg = Table::from_str(
        r#"[compiler-output]
    emit = "sesa""#,
    )
    .unwrap();

    Configurations::parse_compiler_output(&cfg).expect_err("Unrecognized option for `emit`");

    cfg = Table::from_str(
        r#"[compiler-output]
    emit = "sesa""#,
    )
    .unwrap();

    Configurations::parse_compiler_output(&cfg).expect_err("Unrecognized option for `emit`");
}

#[test]
fn parse_package_test() {
    let mut package = r#"[package]
    name = "flipper"
    author = "john doe"
    import_paths = ["path1", "path2"]
    import_maps = ["map_name=path"]
    input_files = ["flipper.sol"]
    contracts = ["flipper"]
"#;

    let mut cfg = Table::from_str(package).unwrap();

    let config = Configurations::parse_package(&cfg).unwrap();
    assert_eq!(config.0, ["flipper.sol"]);
    assert_eq!(config.1, ["flipper".to_owned()].into());
    assert_eq!(config.2, [PathBuf::from("path1"), PathBuf::from("path2")]);
    assert_eq!(config.3, [("map_name".to_owned(), PathBuf::from("path"))]);

    package = r#"[package]
    import_maps = ["map_name=path"]
"#;

    cfg = Table::from_str(package).unwrap();

    Configurations::parse_package(&cfg).expect_err("No input_files defined in .toml file");

    package = r#"[package]
    input_files = ["flipper.sol"]
    import_maps = ["map_name.path"]
"#;
    cfg = Table::from_str(package).unwrap();

    Configurations::parse_package(&cfg).expect_err("contains no '='");
}

#[test]
fn parse_target_test() {
    let mut package = r#"[target]
    name = "substrate"
    value_length = 16
    address_length = 32
"#;

    let mut cfg = Table::from_str(package).unwrap();

    let target = Configurations::parse_target(&cfg).unwrap();
    assert!(
        target.eq(&Target::Substrate {
            address_length: 32,
            value_length: 16
        })
    );

    package = r#"[targett_typo]
    name = "substrate"
"#;
    cfg = Table::from_str(package).unwrap();

    Configurations::parse_target(&cfg)
        .expect_err("no target field defined in toml file. please specify target.");

    package = r#"[target]
    value_length = 16
"#;
    cfg = Table::from_str(package).unwrap();

    Configurations::parse_target(&cfg).expect_err(
        "no `name` field defined for `target`. provide either `solana` or `substrate`.",
    );

    package = r#"[target]
    name = "substrate"
    value_length = "sesa"
"#;
    cfg = Table::from_str(package).unwrap();

    Configurations::parse_target(&cfg).expect_err("Invalid value for key `value_length`");
}

#[test]
fn parse_options_test() {
    let package = r#"[debug-features]
    prints = true
    log-runtime-errors = false
    llvm-IR-debug-info = false
    
    [optimizations]
    dead_storage = true
    constant_folding = true
    strength_reduce = true
    vector_to_slice = true
    common_subexpression_elimination = true
    llvm-IR_optimization_level = "default"
"#;
    let cfg = Table::from_str(package).unwrap();
    let options = Configurations::parse_options(&cfg).unwrap();
    assert_eq!(options, Options::default());
}
}
