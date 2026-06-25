// SPDX-License-Identifier: Apache-2.0

//! Compile-time / sema rejection tests for Soroban bytes.
//! These verify that illegal operations are rejected at compile time
//! rather than silently emitting broken code.
//!
//! Tests will be added one by one as Phase 3 and the coverage backfill land.

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
    let opt = inkwell::OptimizationLevel::Default;

    let (_, ns) = compile(
        tmp_file,
        &mut cache,
        Target::Soroban,
        &Options {
            opt_level: opt.into(),
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

#[test]
fn fixed_bytes_index_assignment_is_rejected() {
    // bytesN[i] = x is illegal in Solidity (fixed bytes are value types).
    let ns = compile_soroban(
        r#"contract T {
    function bad(bytes4 b) public pure returns (bytes4) {
        b[0] = 0x01;
        return b;
    }
}"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|d| d.level == Level::Error)
        .collect::<Vec<_>>();

    assert!(
        !errors.is_empty(),
        "assigning to bytesN[i] must be a sema error"
    );
}

#[test]
fn nested_bytesn_array_abi_is_rejected() {
    // Gap E: bytes32[] as ABI type is not yet supported (nested bytes in array).
    let ns = compile_soroban(
        r#"contract T {
    function f(bytes32[] memory data) public pure returns (uint64) {
        return uint64(data.length);
    }
}"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|d| d.level == Level::Error)
        .collect::<Vec<_>>();

    assert!(
        !errors.is_empty(),
        "bytes32[] memory param must be rejected"
    );
}

#[test]
fn nested_bytes_array_abi_is_rejected() {
    // Gap E: bytes[] as ABI type is not yet supported (nested bytes in array).
    let ns = compile_soroban(
        r#"contract T {
    function f(bytes[] memory data) public pure returns (uint64) {
        return uint64(data.length);
    }
}"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|d| d.level == Level::Error)
        .collect::<Vec<_>>();

    assert!(!errors.is_empty(), "bytes[] memory param must be rejected");
}
