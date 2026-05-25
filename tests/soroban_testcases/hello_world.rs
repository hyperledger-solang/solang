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
        vec!["unknown".to_string()],
        "0.0.1",
    );

    ns
}

#[test]
fn hello_world() {
    let ns = compile_soroban(
        r#"
        contract HelloWorld {
            function hello(string memory to) public pure returns (string[] memory) {
                string[] memory res = new string[](2);
                res[0] = "Hello";
                res[1] = to;
                return res;
            }
        }"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == Level::Error)
        .collect::<Vec<_>>();

    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].message,
        "type 'string[] memory' is not supported as a Soroban external function return value"
    );
}
