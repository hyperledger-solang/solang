// SPDX-License-Identifier: Apache-2.0

use solang::codegen::Options;
use solang::file_resolver::FileResolver;
use solang::sema::ast::Namespace;
use solang::{compile, Target};
use solang_parser::diagnostics::Level;
use std::ffi::OsStr;

fn compile_target(src: &str, target: Target) -> Namespace {
    let tmp_file = OsStr::new("test.sol");
    let mut cache = FileResolver::default();
    cache.set_file_contents(tmp_file.to_str().unwrap(), src.to_string());
    let opt = inkwell::OptimizationLevel::Default;

    let (_, ns) = compile(
        tmp_file,
        &mut cache,
        target,
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

fn compile_soroban(src: &str) -> Namespace {
    compile_target(src, Target::Soroban)
}

#[test]
fn dynamic_bytes_external_parameters_are_supported() {
    let ns = compile_soroban(
        r#"contract test {
    function public_len(bytes memory data) public returns (uint64) {
        return data.length;
    }

    function external_len(bytes memory data) external returns (uint64) {
        return data.length;
    }
}"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == Level::Error)
        .collect::<Vec<_>>();

    assert_eq!(
        errors.len(),
        0,
        "bytes memory parameters should now be supported"
    );
}

#[test]
fn static_bytes_external_parameters_are_supported() {
    // Phase 1: bytesN params and returns are now supported.
    // Runtime coverage is in tests/soroban_testcases/bytes_pass.rs.
    let ns = compile_soroban(
        r#"contract test {
    function echo(bytes32 data) public pure returns (bytes32) { return data; }
    function echo1(bytes1  data) public pure returns (bytes1)  { return data; }
    function echo4(bytes4  data) public pure returns (bytes4)  { return data; }
}"#,
    );

    assert!(
        !ns.diagnostics.any_errors(),
        "bytesN params and returns must compile without errors"
    );
}

#[test]
fn string_external_returns_are_supported() {
    let ns = compile_soroban(
        r#"contract test {
    function public_make() public returns (string memory) {
        return "hello";
    }

    function external_make() external returns (string memory) {
        return "hello";
    }
}"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == Level::Error)
        .collect::<Vec<_>>();

    assert_eq!(
        errors.len(),
        0,
        "string memory returns should now be supported"
    );
}

#[test]
fn bytes_external_returns_are_supported() {
    // Phase 1: both bytes memory and bytes32 returns are now supported.
    let ns = compile_soroban(
        r#"contract test {
    function public_make() public pure returns (bytes memory) {
        return hex"01";
    }

    function external_make() external pure returns (bytes32) {
        return bytes32(uint256(1));
    }
}"#,
    );

    assert!(
        !ns.diagnostics.any_errors(),
        "bytes memory and bytes32 returns must compile without errors"
    );
}

#[test]
fn nested_and_struct_function_abi_types_are_rejected() {
    let ns = compile_soroban(
        r#"contract test {
    struct Item {
        uint64 value;
    }

    function bytes_array(bytes[] memory data) public returns (uint64) {
        return uint64(data.length);
    }

    function struct_param(Item memory item) public returns (uint64) {
        return item.value;
    }

    function struct_return() public returns (Item memory) {
        return Item({ value: 1 });
    }

    function array_return() public returns (uint64[] memory) {
        uint64[] memory values = new uint64[](1);
        values[0] = 1;
        return values;
    }

    function multiple_returns() public returns (uint64, uint64) {
        return (1, 2);
    }
}"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == Level::Error)
        .collect::<Vec<_>>();

    assert_eq!(errors.len(), 5);
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'bytes[] memory' is not supported as a Soroban external function parameter"));
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'struct test.Item memory' is not supported as a Soroban external function parameter"));
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'struct test.Item memory' is not supported as a Soroban external function return value"));
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'uint64[] memory' is not supported as a Soroban external function return value"));
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "Soroban external functions can return at most one value"));
}

#[test]
fn struct_public_accessor_is_rejected() {
    // bytes/bytesN are now supported (Phase 3). Only multi-field struct accessors remain rejected.
    let ns = compile_soroban(
        r#"contract test {
    struct Pair { uint64 first; uint64 second; }
    Pair public pair;
}"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == Level::Error)
        .collect::<Vec<_>>();

    assert_eq!(errors.len(), 1);
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'struct test.Pair' is not supported as a Soroban public variable accessor return value"));
}

#[test]
fn unsupported_event_parameters_are_rejected() {
    // bytes/bytesN events are now supported (Phase 3). Only struct event params remain rejected.
    let ns = compile_soroban(
        r#"contract test {
    event Struct(Item data);
    struct Item { uint64 value; }
    function go() public { emit Struct(Item({ value: 1 })); }
}"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == Level::Error)
        .collect::<Vec<_>>();

    assert_eq!(errors.len(), 1);
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'struct test.Item' is not supported as a Soroban event parameter"));
}

#[test]
fn internal_private_dynamic_bytes_parameters_are_allowed() {
    let ns = compile_soroban(
        r#"contract test {
    function entry() public returns (uint64) {
        bytes memory data = new bytes(2);
        return internal_len(data) + private_len(data);
    }

    function internal_len(bytes memory data) internal returns (uint64) {
        return data.length;
    }

    function private_len(bytes memory data) private returns (uint64) {
        return data.length;
    }
}"#,
    );

    assert!(!ns.diagnostics.any_errors());
}

#[test]
fn known_good_soroban_contract_still_compiles() {
    let ns = compile_soroban(
        r#"contract test {
    function roundtrip(uint64 value) public returns (uint64) {
        return value;
    }
}"#,
    );

    assert!(!ns.diagnostics.any_errors());
}

#[test]
fn public_string_accessors_are_allowed() {
    let ns = compile_soroban(
        r#"contract test {
    string public name;
}"#,
    );

    assert!(!ns.diagnostics.any_errors());
}

#[test]
fn bytes_storage_subscript_assignment_compiles() {
    let ns = compile_soroban(
        r#"contract test {
    bytes data;

    function write_data() public {
        data[0] = 0x01;
    }
}"#,
    );

    assert!(
        !ns.diagnostics.any_errors(),
        "storage bytes subscript assignment must now compile without errors"
    );
}

#[test]
fn soroban_string_helper_patterns_are_supported() {
    // All of these patterns were previously rejected; they now compile correctly because
    // strings are struct.vector (same as DynamicBytes) and can be passed/cast freely.
    let ns = compile_soroban(
        r#"contract test {
    function helper(string memory data) internal returns (uint64) {
        return uint64(bytes(data).length);
    }

    function call_helper_with_literal() public returns (uint64) {
        return helper("abc");
    }

    function make() internal returns (string memory) {
        return "abc";
    }

    function returned_string_length() public returns (uint64) {
        return uint64(bytes(make()).length);
    }

    function local_string_length() public returns (uint64) {
        string memory data = "abc";
        return uint64(bytes(data).length);
    }
}"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == Level::Error)
        .collect::<Vec<_>>();

    assert_eq!(errors.len(), 0, "string helper patterns should now compile");
}

#[test]
fn non_soroban_string_helpers_do_not_get_soroban_diagnostics() {
    let ns = compile_target(
        r#"contract test {
    function helper(string memory data) internal returns (uint64) {
        return uint64(bytes(data).length);
    }

    function call_helper_with_literal() public returns (uint64) {
        return helper("abc");
    }
}"#,
        Target::default_polkadot(),
    );

    assert!(!ns
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("target soroban")));
}
