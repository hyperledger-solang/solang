use crate::{build_solidity, SorobanEnv};
use soroban_sdk::testutils::Logs;
use soroban_sdk::{IntoVal, Val};

#[test]
fn test_constructor_increments_count() {
    let wasm = build_solidity(
        r#"contract counter {
            uint64 public count = 1;

            constructor() {
                count += 1;
            }

            function get() public view returns (uint64) {
                return count;
            }
        }"#,
    );
    let mut src = SorobanEnv::new();
    // No constructor arguments
    let constructor_args: soroban_sdk::Vec<Val> = soroban_sdk::Vec::new(&src.env);
    let address = src.register_contract(wasm, constructor_args);

    let res = src.invoke_contract(&address, "get", vec![]);
    let expected: Val = 2_u64.into_val(&src.env);
    assert!(
        expected.shallow_eq(&res),
        "expected: {:?}, got: {:?}",
        expected,
        res
    );
}

#[test]
fn test_constructor_logs_message_on_call() {
    let wasm = build_solidity(
        r#"contract counter {
            uint64 public count = 1;

            constructor() {
                print("Constructor called");
            }

            function get() public view returns (uint64) {
                return count;
            }
        }"#,
    );
    let mut src = SorobanEnv::new();
    // No constructor arguments
    let constructor_args: soroban_sdk::Vec<Val> = soroban_sdk::Vec::new(&src.env);
    let address = src.register_contract(wasm, constructor_args);

    let _res = src.invoke_contract(&address, "get", vec![]);

    let logs = src.env.logs().all();
    assert!(logs[0].contains("Constructor called"));
}

// FIXME: Uncomment this test once the constructor arguments are supported
// #[test]
fn _test_constructor_set_count_value() {
    let wasm = build_solidity(
        r#"contract counter {
            uint64 public count = 1;

            constructor(uint64 initial_count) {
                count = initial_count;
            }

            function get() public view returns (uint64) {
                return count;
            }
        }"#,
    );
    let mut src = SorobanEnv::new();
    let mut constructor_args: soroban_sdk::Vec<Val> = soroban_sdk::Vec::new(&src.env);
    constructor_args.push_back(42_u64.into_val(&src.env));
    let address = src.register_contract(wasm, constructor_args);

    // Get the value of count and check it is 42
    let res = src.invoke_contract(&address, "get", vec![]);
    let expected: Val = 42_u64.into_val(&src.env);

    assert!(
        expected.shallow_eq(&res),
        "expected: {:?}, got: {:?}",
        expected,
        res
    );
}
