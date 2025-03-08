// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{IntoVal, Val};

#[test]
fn math() {
    let runtime = build_solidity(
        r#"contract math {
        function max(uint64 a, uint64 b) public returns (uint64) {
            if (a > b) {
                return a;
            } else {
                return b;
            }
        }
    }"#,
        |_| {},
    );

    let arg: Val = 5_u64.into_val(&runtime.env);
    let arg2: Val = 4_u64.into_val(&runtime.env);

    let addr = runtime.contracts.last().unwrap();
    let res = runtime.invoke_contract(addr, "max", vec![arg, arg2]);

    let expected: Val = 5_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn math_same_name() {
    let src = build_solidity(
        r#"contract math {
        function max(uint64 a, uint64 b) public returns (uint64) {
            if (a > b) {
                return a;
            } else {
                return b;
            }
        }
    
        function max(uint64 a, uint64 b, uint64 c) public returns (uint64) {
            if (a > b) {
                if (a > c) {
                    return a;
                } else {
                    return c;
                }
            } else {
                if (b > c) {
                    return b;
                } else {
                    return c;
                }
            }
        }
    }
    "#,
        |_| {},
    );

    let addr = src.contracts.last().unwrap();

    let arg1: Val = 5_u64.into_val(&src.env);
    let arg2: Val = 4_u64.into_val(&src.env);
    let res = src.invoke_contract(addr, "max_uint64_uint64", vec![arg1, arg2]);
    let expected: Val = 5_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&res));

    let arg1: Val = 5_u64.into_val(&src.env);
    let arg2: Val = 4_u64.into_val(&src.env);
    let arg3: Val = 6_u64.into_val(&src.env);
    let res = src.invoke_contract(addr, "max_uint64_uint64_uint64", vec![arg1, arg2, arg3]);
    let expected: Val = 6_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn i128_ops() {
    let runtime = build_solidity(
        r#"contract math {
        function add(int128 a, int128 b) public returns (int128) {
            return a + b;
        }

        function sub(int128 a, int128 b) public returns (int128) {
            return a - b;
        }

        function mul(int128 a, int128 b) public returns (int128) {
            return a * b;
        }

        function div(int128 a, int128 b) public returns (int128) {
            return a / b;
        }

        function mod(int128 a, int128 b) public returns (int128) {
            return a % b;
        }
    }"#,
        |_| {},
    );

    let arg: Val = 5_i128.into_val(&runtime.env);
    let arg2: Val = 4_i128.into_val(&runtime.env);

    let addr = runtime.contracts.last().unwrap();
    let res = runtime.invoke_contract(addr, "add", vec![arg, arg2]);

    let expected: Val = 9_i128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "sub", vec![arg, arg2]);

    let expected: Val = 1_i128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mul", vec![arg, arg2]);

    let expected: Val = 20_i128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "div", vec![arg, arg2]);

    let expected: Val = 1_i128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mod", vec![arg, arg2]);

    let expected: Val = 1_i128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn u128_ops() {
    let runtime = build_solidity(
        r#"contract math {
        function add(uint128 a, uint128 b) public returns (uint128) {
            return a + b;
        }

        function sub(uint128 a, uint128 b) public returns (uint128) {
            return a - b;
        }

        function mul(uint128 a, uint128 b) public returns (uint128) {
            return a * b;
        }

        function div(uint128 a, uint128 b) public returns (uint128) {
            return a / b;
        }

        function mod(uint128 a, uint128 b) public returns (uint128) {
            return a % b;
        }
    }"#,
        |_| {},
    );

    let arg: Val = 5_u128.into_val(&runtime.env);
    let arg2: Val = 4_u128.into_val(&runtime.env);

    let addr = runtime.contracts.last().unwrap();
    let res = runtime.invoke_contract(addr, "add", vec![arg, arg2]);

    let expected: Val = 9_u128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "sub", vec![arg, arg2]);

    let expected: Val = 1_u128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mul", vec![arg, arg2]);

    let expected: Val = 20_u128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "div", vec![arg, arg2]);

    let expected: Val = 1_u128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(addr, "mod", vec![arg, arg2]);

    let expected: Val = 1_u128.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}
