// SPDX-License-Identifier: Apache-2.0

use solang::codegen::Options;
use solang::file_resolver::FileResolver;
use solang::sema::ast::Namespace;
use solang::{compile, Target};
use solang_parser::diagnostics::Level;
use std::ffi::OsStr;

fn compile_soroban(src: &str) -> Namespace {
    let tmp_file = OsStr::new("test.sol");
    let mut cache = FileResolver::default();
    cache.set_file_contents(tmp_file.to_str().unwrap(), src.to_string());

    let (_, ns) = compile(
        tmp_file,
        &mut cache,
        Target::Soroban,
        &Options {
            opt_level: inkwell::OptimizationLevel::Default.into(),
            log_runtime_errors: true,
            log_prints: true,
            #[cfg(feature = "wasm_opt")]
            wasm_opt: Some(contract_build::OptimizationPasses::Z),
            soroban_version: None,
            ..Default::default()
        },
        std::vec!["unknown".to_string()],
        "0.0.1",
    );

    ns
}

fn assert_inline_assembly_rejected(ns: &Namespace) {
    let errors = ns
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == Level::Error)
        .collect::<Vec<_>>();

    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].message,
        "inline assembly is not supported on Soroban"
    );
}

#[test]
fn inline_assembly_with_builtin_is_rejected() {
    let ns = compile_soroban(
        r#"contract C {
    function add(uint64 a, uint64 b) public pure returns (uint64 r) {
        assembly { r := add(a, b) }
    }
}"#,
    );

    assert_inline_assembly_rejected(&ns);
}

#[test]
fn inline_assembly_without_builtin_is_rejected() {
    let ns = compile_soroban(
        r#"contract C {
    function f() public pure returns (uint64 r) {
        assembly {
            let x := 1
            r := x
        }
    }
}"#,
    );

    assert_inline_assembly_rejected(&ns);
}
