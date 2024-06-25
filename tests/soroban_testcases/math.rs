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
