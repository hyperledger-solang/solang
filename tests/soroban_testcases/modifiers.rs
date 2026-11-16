// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{FromVal, IntoVal};

const SRC: &str = r#"contract C {
    uint64 count;

    modifier positive(uint64 x) {
        require(x > 0, "x must be positive");
        _;
    }

    function add(uint64 x) public positive(x) returns (uint64) {
        count += x;
        return count;
    }
}"#;

#[test]
fn modifier_runs_before_body() {
    let runtime = build_solidity(SRC, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    // the check passes, so the body runs, and storage is kept between calls
    let res = runtime.invoke_contract(addr, "add", vec![5_u64.into_val(env)]);
    assert_eq!(u64::from_val(env, &res), 5);
    let res = runtime.invoke_contract(addr, "add", vec![3_u64.into_val(env)]);
    assert_eq!(u64::from_val(env, &res), 8);

    // the check fails, so the call reverts before the body runs
    let logs = runtime.invoke_contract_expect_error(addr, "add", vec![0_u64.into_val(env)]);
    assert!(
        logs.iter()
            .any(|log| log.contains("require condition failed")),
        "{logs:?}"
    );
}

#[test]
fn stacked_modifiers_all_run() {
    let runtime = build_solidity(
        r#"contract C {
        modifier positive(uint64 x) {
            require(x > 0);
            _;
        }

        modifier small(uint64 x) {
            require(x < 100);
            _;
        }

        function double(uint64 x) public pure positive(x) small(x) returns (uint64) {
            return x * 2;
        }
    }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "double", vec![5_u64.into_val(env)]);
    assert_eq!(u64::from_val(env, &res), 10);

    // each modifier can stop the call
    for bad in [0_u64, 100] {
        let logs = runtime.invoke_contract_expect_error(addr, "double", vec![bad.into_val(env)]);
        assert!(
            logs.iter()
                .any(|log| log.contains("require condition failed")),
            "{logs:?}"
        );
    }
}

#[test]
fn unused_modifier_compiles() {
    let runtime = build_solidity(
        r#"contract C {
        modifier positive(uint64 x) {
            require(x > 0);
            _;
        }

        function get() public pure returns (uint64) {
            return 7;
        }
    }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();

    let res = runtime.invoke_contract(addr, "get", vec![]);
    assert_eq!(u64::from_val(&runtime.env, &res), 7);
}

/// Function numbers count every function in the file, cfg numbers count only this contract's
/// cfgs. The interface's functions come first, so `add` has a function number that is not its
/// cfg number; the public entry must call `add` by its cfg number.
#[test]
fn call_by_cfg_number_not_function_number() {
    let runtime = build_solidity(
        r#"interface I {
        function one() external returns (uint64);
        function two(uint64 x) external returns (uint64);
    }

    contract C {
        function add(uint64 x, uint64 y) public pure returns (uint64) {
            return x + y;
        }
    }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let args = vec![2_u64.into_val(env), 3_u64.into_val(env)];
    let res = runtime.invoke_contract(addr, "add", args);
    assert_eq!(u64::from_val(env, &res), 5);
}
