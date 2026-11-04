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

fn assert_compiles(src: &str) {
    let ns = compile_soroban(src);

    assert!(
        !ns.diagnostics.iter().any(|d| d.level == Level::Error),
        "unexpected errors: {:?}",
        ns.diagnostics
    );
}

/// Regression test: a public function with a modifier and a named return value
/// used to hit `unreachable!()` in the Soroban dispatch and panic the compiler.
#[test]
fn modifier_with_named_return() {
    assert_compiles(
        r#"
        contract Repro {
            uint128 public total;
            modifier guard() { _; }
            function inc(uint128 amount) public guard returns (uint128 shares) {
                shares = amount;
                total = total + shares;
                return shares;
            }
        }
        "#,
    );
}

#[test]
fn modifier_with_unnamed_return() {
    assert_compiles(
        r#"
        contract Repro {
            uint128 public total;
            modifier guard() { _; }
            function inc(uint128 amount) public guard returns (uint128) {
                total = total + amount;
                return total;
            }
        }
        "#,
    );
}
