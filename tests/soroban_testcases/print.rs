// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, SorobanEnv};
use soroban_sdk::{testutils::Logs, Val};

#[test]
fn log_runtime_error() {
    let wasm = build_solidity(
        r#"contract counter {
            uint64 public count = 1;
        
            function decrement() public returns (uint64){
                count -= 1;
                return count;
            }
        }"#,
    );

    let mut runtime = SorobanEnv::new();
    // No constructor arguments
    let constructor_args: soroban_sdk::Vec<Val> = soroban_sdk::Vec::new(&runtime.env);
    let addr = runtime.register_contract(wasm, constructor_args);

    runtime.invoke_contract(&addr, "decrement", vec![]);

    let logs = runtime.invoke_contract_expect_error(&addr, "decrement", vec![]);

    assert!(logs[0].contains("runtime_error: math overflow in test.sol:5:17-27"));
}

#[test]
fn print() {
    let wasm = build_solidity(
        r#"contract Printer {

            function print() public {
                print("Hello, World!");
            }
        }"#,
    );

    let mut runtime = SorobanEnv::new();
    // No constructor arguments
    let constructor_args: soroban_sdk::Vec<Val> = soroban_sdk::Vec::new(&runtime.env);
    let addr = runtime.register_contract(wasm, constructor_args);

    runtime.invoke_contract(&addr, "print", vec![]);

    let logs = runtime.env.logs().all();

    assert!(logs[0].contains("Hello, World!"));
}

#[test]
fn print_then_runtime_error() {
    let wasm = build_solidity(
        r#"contract counter {
            uint64 public count = 1;
        
            function decrement() public returns (uint64){
                print("Second call will FAIL!");
                count -= 1;
                return count;
            }
        }"#,
    );

    let mut runtime = SorobanEnv::new();
    // No constructor arguments
    let constructor_args: soroban_sdk::Vec<Val> = soroban_sdk::Vec::new(&runtime.env);
    let addr = runtime.register_contract(wasm, constructor_args);

    runtime.invoke_contract(&addr, "decrement", vec![]);

    let logs = runtime.invoke_contract_expect_error(&addr, "decrement", vec![]);

    assert!(logs[0].contains("Second call will FAIL!"));
    assert!(logs[1].contains("runtime_error: math overflow in test.sol:6:17-27"));
}
