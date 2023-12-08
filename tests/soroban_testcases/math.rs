// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::Val;

#[test]
fn math() {
    let env = build_solidity(
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

    let addr = env.contracts.last().unwrap();
    let res = env.invoke_contract(
        addr,
        "max",
        vec![*Val::from_u32(4).as_val(), *Val::from_u32(5).as_val()],
    );
    assert!(Val::from_u32(5).as_val().shallow_eq(&res))
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
    let res = src.invoke_contract(
        addr,
        "max_uint64_uint64",
        vec![*Val::from_u32(4).as_val(), *Val::from_u32(5).as_val()],
    );
    assert!(Val::from_u32(5).as_val().shallow_eq(&res));

    let res = src.invoke_contract(
        addr,
        "max_uint64_uint64_uint64",
        vec![
            *Val::from_u32(4).as_val(),
            *Val::from_u32(5).as_val(),
            *Val::from_u32(6).as_val(),
        ],
    );
    assert!(Val::from_u32(6).as_val().shallow_eq(&res));
}
