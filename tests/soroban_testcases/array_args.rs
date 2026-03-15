// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{IntoVal, Val};

#[test]
fn array_argument_ops() {
    let runtime = build_solidity(
        r#"
        contract array_arg_ops {
            function sum_len(uint64[] memory arr) public returns (uint64) {
                uint64 sum = 0;

                for (uint64 i = 0; i < arr.length; i++) {
                    sum += arr[i];
                }

                return sum + arr.length;
            }

            function mutate_and_read(uint64[] memory arr) public returns (uint64) {
                arr[0] = arr[0] + 10;
                arr[1] = arr[1] * 2;

                return arr[0] + arr[1] + arr.length;
            }

            function pair_sum_at(uint64[] memory arr, uint64 i) public returns (uint64) {
                return arr[i] + arr[i + 1];
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let arr: soroban_sdk::Vec<u64> = soroban_sdk::vec![&runtime.env, 1_u64, 2_u64, 3_u64];

    // sum_len([1,2,3]) => 1 + 2 + 3 + len(3) = 9
    let expected: Val = 9_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "sum_len", vec![arr.clone().into_val(&runtime.env)]);
    println!("sum_len result: {:?}", res);
    assert!(expected.shallow_eq(&res));

    // mutate_and_read([1,2,3]) => (1+10) + (2*2) + 3 = 18
    let expected: Val = 18_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(
        addr,
        "mutate_and_read",
        vec![arr.clone().into_val(&runtime.env)],
    );
    assert!(expected.shallow_eq(&res));

    // pair_sum_at([1,2,3], 1) => 2 + 3 = 5
    let expected: Val = 5_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(
        addr,
        "pair_sum_at",
        vec![arr.into_val(&runtime.env), 1_u64.into_val(&runtime.env)],
    );
    assert!(expected.shallow_eq(&res));
}

#[test]
fn array_argument_storage_roundtrip() {
    let runtime = build_solidity(
        r#"
        contract array_arg_ops {
            uint64[] public stored_array;

            function store_and_get(uint64[] memory arr, uint64 index) public returns (uint64) {
                stored_array = arr;
                return stored_array[index];
            }

            function store_and_weighted_sum(uint64[] memory arr) public returns (uint64) {
                stored_array = arr;

                uint64 acc = 0;
                for (uint64 i = 0; i < stored_array.length; i++) {
                    acc += stored_array[i] * (i + 1);
                }

                return acc;
            }

            function probe() public returns (uint64) {
                return stored_array[0] + stored_array[stored_array.length - 1] + stored_array.length;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();
    let arr1: soroban_sdk::Vec<u64> = soroban_sdk::vec![&runtime.env, 11_u64, 22_u64, 33_u64];
    let arr2: soroban_sdk::Vec<u64> = soroban_sdk::vec![&runtime.env, 2_u64, 4_u64, 6_u64, 8_u64];

    // store_and_get([11,22,33], 1) => 22
    let expected: Val = 22_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(
        addr,
        "store_and_get",
        vec![arr1.into_val(&runtime.env), 1_u64.into_val(&runtime.env)],
    );
    assert!(expected.shallow_eq(&res));

    // store_and_weighted_sum([2,4,6,8]) => 2*1 + 4*2 + 6*3 + 8*4 = 60
    let expected: Val = 60_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(
        addr,
        "store_and_weighted_sum",
        vec![arr2.into_val(&runtime.env)],
    );
    assert!(expected.shallow_eq(&res));

    // probe() uses persisted storage from previous call: 2 + 8 + 4 = 14
    let expected: Val = 14_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "probe", vec![]);
    assert!(expected.shallow_eq(&res));
}
