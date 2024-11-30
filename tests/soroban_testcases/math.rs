// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, SorobanEnv};
use soroban_sdk::{IntoVal, Val};

#[test]
fn math() {
    let wasm = build_solidity(
        r#"contract math {
        function max(uint64 a, uint64 b) public returns (uint64) {
            if (a > b) {
                return a;
            } else {
                return b;
            }
        }
    }"#,
    );
    let mut env = SorobanEnv::new();
    // No constructor arguments
    let constructor_args: soroban_sdk::Vec<Val> = soroban_sdk::Vec::new(&env.env);
    let address = env.register_contract(wasm, constructor_args);

    let arg: Val = 5_u64.into_val(&env.env);
    let arg2: Val = 4_u64.into_val(&env.env);

    let res = env.invoke_contract(&address, "max", vec![arg, arg2]);

    let expected: Val = 5_u64.into_val(&env.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn math_same_name() {
    let wasm = build_solidity(
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
    );
    let mut src = SorobanEnv::new();
    // No constructor arguments
    let constructor_args: soroban_sdk::Vec<Val> = soroban_sdk::Vec::new(&src.env);
    let address = src.register_contract(wasm, constructor_args);

    let arg1: Val = 5_u64.into_val(&src.env);
    let arg2: Val = 4_u64.into_val(&src.env);
    let res = src.invoke_contract(&address, "max_uint64_uint64", vec![arg1, arg2]);
    let expected: Val = 5_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&res));

    let arg1: Val = 5_u64.into_val(&src.env);
    let arg2: Val = 4_u64.into_val(&src.env);
    let arg3: Val = 6_u64.into_val(&src.env);
    let res = src.invoke_contract(&address, "max_uint64_uint64_uint64", vec![arg1, arg2, arg3]);
    let expected: Val = 6_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&res));
}
