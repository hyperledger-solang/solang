// SPDX-License-Identifier: Apache-2.0

use solang::codegen::Options;
use solang::file_resolver::FileResolver;
use solang::sema::ast::Namespace;
use solang::sema::file::PathDisplay;
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
fn dynamic_bytes_external_parameters_are_rejected() {
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

    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|diagnostic| diagnostic.message
        == "type 'bytes memory' is not supported as a Soroban external function parameter"));

    let locations = errors
        .iter()
        .map(|diagnostic| ns.loc_to_string(PathDisplay::None, &diagnostic.loc))
        .collect::<Vec<_>>();

    assert!(locations.iter().any(|loc| loc.starts_with("2:")));
    assert!(locations.iter().any(|loc| loc.starts_with("6:")));
}

#[test]
fn static_bytes_external_parameters_are_rejected() {
    let ns = compile_soroban(
        r#"contract test {
    function len(bytes32 data) public returns (uint64) {
        return uint64(data.length);
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
        "type 'bytes32' is not supported as a Soroban external function parameter"
    );
    assert!(ns
        .loc_to_string(PathDisplay::None, &errors[0].loc)
        .starts_with("2:"));
}

#[test]
fn string_external_returns_are_rejected() {
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

    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|diagnostic| diagnostic.message
        == "type 'string memory' is not supported as a Soroban external function return value"));

    let locations = errors
        .iter()
        .map(|diagnostic| ns.loc_to_string(PathDisplay::None, &diagnostic.loc))
        .collect::<Vec<_>>();

    assert!(locations.iter().any(|loc| loc.starts_with("2:")));
    assert!(locations.iter().any(|loc| loc.starts_with("6:")));
}

#[test]
fn bytes_external_returns_are_rejected() {
    let ns = compile_soroban(
        r#"contract test {
    function public_make() public returns (bytes memory) {
        return hex"01";
    }

    function external_make() external returns (bytes32) {
        return bytes32(uint256(1));
    }
}"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == Level::Error)
        .collect::<Vec<_>>();

    assert_eq!(errors.len(), 2);
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'bytes memory' is not supported as a Soroban external function return value"));
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'bytes32' is not supported as a Soroban external function return value"));

    let locations = errors
        .iter()
        .map(|diagnostic| ns.loc_to_string(PathDisplay::None, &diagnostic.loc))
        .collect::<Vec<_>>();

    assert!(locations.iter().any(|loc| loc.starts_with("2:")));
    assert!(locations.iter().any(|loc| loc.starts_with("6:")));
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
fn bytes_public_accessors_are_rejected() {
    let ns = compile_soroban(
        r#"contract test {
    struct Pair {
        uint64 first;
        uint64 second;
    }

    bytes public dynamic_data;
    bytes32 public fixed_data;
    bytes1 public tiny_data;
    mapping(address => bytes) public keyed_data;
    Pair public pair;
}"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == Level::Error)
        .collect::<Vec<_>>();

    assert_eq!(errors.len(), 5);
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'bytes' is not supported as a Soroban public variable accessor return value"));
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'bytes32' is not supported as a Soroban public variable accessor return value"));
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'bytes1' is not supported as a Soroban public variable accessor return value"));
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'struct test.Pair' is not supported as a Soroban public variable accessor return value"));

    let locations = errors
        .iter()
        .map(|diagnostic| ns.loc_to_string(PathDisplay::None, &diagnostic.loc))
        .collect::<Vec<_>>();

    assert!(locations.iter().any(|loc| loc.starts_with("7:")));
    assert!(locations.iter().any(|loc| loc.starts_with("8:")));
    assert!(locations.iter().any(|loc| loc.starts_with("9:")));
    assert!(locations.iter().any(|loc| loc.starts_with("10:")));
    assert!(locations.iter().any(|loc| loc.starts_with("11:")));
}

#[test]
fn unsupported_event_parameters_are_rejected() {
    let ns = compile_soroban(
        r#"contract test {
    event Dynamic(bytes data);
    event Fixed(bytes32 data);
    event Struct(Item data);

    struct Item {
        uint64 value;
    }

    function emit_events() public {
        bytes memory data = hex"01";
        emit Dynamic(data);
        emit Fixed(bytes32(uint256(1)));
        emit Struct(Item({ value: 1 }));
    }
}"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == Level::Error)
        .collect::<Vec<_>>();

    assert_eq!(errors.len(), 3);
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'bytes' is not supported as a Soroban event parameter"));
    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "type 'bytes32' is not supported as a Soroban event parameter"));
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
fn unsupported_soroban_storage_codegen_paths_are_rejected_before_emit() {
    let ns = compile_soroban(
        r#"contract test {
    bytes data;

    function write_data() public {
        data[0] = 0x01;
    }
}"#,
    );

    let errors = ns
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level == Level::Error)
        .collect::<Vec<_>>();

    assert!(errors.iter().any(|diagnostic| diagnostic.message
        == "storage bytes subscript assignment is not supported for target soroban"));
}

#[test]
fn unsafe_soroban_string_helper_paths_are_rejected() {
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

    assert_eq!(errors.len(), 3);
    assert!(errors.iter().any(|diagnostic| {
        diagnostic.message
        == "passing string memory values to internal functions is not supported for target soroban"
    }));
    assert_eq!(
        errors
            .iter()
            .filter(|diagnostic| diagnostic.message
                == "using string memory as bytes is not supported for target soroban")
            .count(),
        2
    );
    assert!(errors.iter().all(|diagnostic| ns
        .loc_to_string(PathDisplay::None, &diagnostic.loc)
        .split(':')
        .next()
        .is_some_and(|line| line.parse::<usize>().is_ok())));
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
