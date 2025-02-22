// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::testutils::Logs;

#[test]
fn log_runtime_error() {
    let src = build_solidity(
        r#"contract counter {
            uint64 public count = 1;
        
            function decrement() public returns (uint64){
                count -= 1;
                return count;
            }
        }"#,
        |_| {},
    );

    let addr = src.contracts.last().unwrap();

    src.invoke_contract(addr, "decrement", vec![]);

    let logs = src.invoke_contract_expect_error(addr, "decrement", vec![]);

    assert!(logs[0].contains("runtime_error: math overflow in test.sol:5:17-27"));
}

#[test]
fn print() {
    let src = build_solidity(
        r#"contract Printer {

            function print() public {
                print("Hello, World!");
            }
        }"#,
        |_| {},
    );

    let addr = src.contracts.last().unwrap();

    src.invoke_contract(addr, "print", vec![]);

    let logs = src.env.logs().all();

    assert!(logs[0].contains("Hello, World!"));
}

#[test]
fn print_then_runtime_error() {
    let src = build_solidity(
        r#"contract counter {
            uint64 public count = 1;
        
            function decrement() public returns (uint64){
                print("Second call will FAIL!");
                count -= 1;
                return count;
            }
        }"#,
        |_| {},
    );

    let addr = src.contracts.last().unwrap();

    src.invoke_contract(addr, "decrement", vec![]);

    let logs = src.invoke_contract_expect_error(addr, "decrement", vec![]);

    assert!(logs[0].contains("Second call will FAIL!"));
    assert!(logs[1].contains("runtime_error: math overflow in test.sol:6:17-27"));
}
