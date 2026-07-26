// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{FromVal, IntoVal};

#[test]
fn returns_freshly_built_array() {
    let runtime = build_solidity(
        r#"
        contract array_return {
            function nums() public pure returns (uint64[] memory) {
                uint64[] memory a = new uint64[](3);
                a[0] = 10;
                a[1] = 20;
                a[2] = 30;
                return a;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    // nums() => [10, 20, 30]
    let ret: soroban_sdk::Vec<u64> =
        soroban_sdk::Vec::from_val(env, &runtime.invoke_contract(addr, "nums", vec![]));
    let expected: soroban_sdk::Vec<u64> = soroban_sdk::vec![env, 10_u64, 20_u64, 30_u64];
    assert_eq!(ret, expected);
}

#[test]
fn round_trips_array_arg_and_return() {
    let runtime = build_solidity(
        r#"
        contract array_return {
            function doubled(uint64[] memory input) public pure returns (uint64[] memory) {
                uint64[] memory out = new uint64[](input.length);
                for (uint64 i = 0; i < input.length; i++) {
                    out[i] = input[i] * 2;
                }
                return out;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    // doubled([1, 2, 3]) => [2, 4, 6]
    let input: soroban_sdk::Vec<u64> = soroban_sdk::vec![env, 1_u64, 2_u64, 3_u64];
    let ret: soroban_sdk::Vec<u64> = soroban_sdk::Vec::from_val(
        env,
        &runtime.invoke_contract(addr, "doubled", vec![input.into_val(env)]),
    );
    let expected: soroban_sdk::Vec<u64> = soroban_sdk::vec![env, 2_u64, 4_u64, 6_u64];
    assert_eq!(ret, expected);
}
