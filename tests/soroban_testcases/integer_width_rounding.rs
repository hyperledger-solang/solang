// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{IntoVal, Val};

#[test]
fn test_int56_rounds_to_int64() {
    let runtime = build_solidity(
        r#"contract test {
        function test_int56(int56 a) public returns (int64) {
            return int64(a);
        }
    }"#,
        |_| {},
    );

    // Check that the function compiles and works with the rounded type
    let arg: Val = 42_i64.into_val(&runtime.env);
    let addr = runtime.contracts.last().unwrap();
    let res = runtime.invoke_contract(addr, "test_int56", vec![arg]);

    let expected: Val = 42_i64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn test_uint56_rounds_to_uint64() {
    let runtime = build_solidity(
        r#"contract test {
        function test_uint56(uint56 a) public returns (uint64) {
            return uint64(a);
        }
    }"#,
        |_| {},
    );

    let arg: Val = 42_u64.into_val(&runtime.env);
    let addr = runtime.contracts.last().unwrap();
    let res = runtime.invoke_contract(addr, "test_uint56", vec![arg]);

    let expected: Val = 42_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn test_int96_rounds_to_int128() {
    let runtime = build_solidity(
        r#"contract test {
        function test_int96(int96 a) public returns (int128) {
            return int128(a);
        }
    }"#,
        |_| {},
    );

    let arg: Val = 42_i128.into_val(&runtime.env);
    let addr = runtime.contracts.last().unwrap();
    let res = runtime.invoke_contract(addr, "test_int96", vec![arg]);

    let expected: Val = 42_i128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn test_uint96_rounds_to_uint128() {
    let runtime = build_solidity(
        r#"contract test {
        function test_uint96(uint96 a) public returns (uint128) {
            return uint128(a);
        }
    }"#,
        |_| {},
    );

    let arg: Val = 42_u128.into_val(&runtime.env);
    let addr = runtime.contracts.last().unwrap();
    let res = runtime.invoke_contract(addr, "test_uint96", vec![arg]);

    let expected: Val = 42_u128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}
