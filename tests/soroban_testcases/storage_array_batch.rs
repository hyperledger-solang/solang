// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{IntoVal, Val};

#[test]
fn storage_array_push_i32_test() {
    let contract_src = r#"
        contract storage_array_i32 {
            int32[] mylist;

            function push_sum() public returns (int32) {
                mylist.push(5);
                mylist.push(10);
                mylist.push(15);
                return mylist[0] + mylist[1] + mylist[2];
            }
        }
    "#;

    let runtime = build_solidity(contract_src, |_| {});

    let addr = runtime.contracts.last().unwrap();
    let expected: Val = 30_i32.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "push_sum", vec![]);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn storage_array_pop_i32_test() {
    let contract_src = r#"
        contract storage_array_pop_i32 {
            int32[] mylist;

            function pop_len() public returns (uint32) {
                mylist.push(5);
                mylist.push(10);
                mylist.push(15);
                mylist.pop();
                return uint32(mylist.length);
            }

            function pop_values() public returns (int32) {
                mylist.push(5);
                mylist.push(10);
                mylist.push(15);
                mylist.pop();
                return mylist[0] + mylist[1];
            }
        }
    "#;

    let mut runtime = build_solidity(contract_src, |_| {});

    let addr = runtime.contracts.last().unwrap();
    let expected: Val = 2_u32.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "pop_len", vec![]);
    assert!(expected.shallow_eq(&res));

    let addr2 = runtime.deploy_contract(contract_src);
    let expected: Val = 15_i32.into_val(&runtime.env);
    let res = runtime.invoke_contract(&addr2, "pop_values", vec![]);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn storage_array_length_i32_test() {
    let contract_src = r#"
        contract storage_array_len_i32 {
            int32[] mylist;

            function len_after_push() public returns (uint32) {
                mylist.push(5);
                mylist.push(10);
                mylist.push(15);
                return uint32(mylist.length); 
            }

            function len_empty() public returns (uint32) {
                return uint32(mylist.length);
            }

            function subscript_sum() public returns (int32) {
                mylist.push(5);
                mylist.push(10);
                mylist.push(15);
                return mylist[0] + mylist[1] + mylist[2];
            }
        }
    "#;

    let mut runtime = build_solidity(contract_src, |_| {});

    let addr = runtime.contracts.last().unwrap();
    let expected: Val = 3_u32.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "len_after_push", vec![]);
    assert!(expected.shallow_eq(&res));

    let addr2 = runtime.deploy_contract(contract_src);
    let expected: Val = 0_u32.into_val(&runtime.env);
    let res = runtime.invoke_contract(&addr2, "len_empty", vec![]);
    assert!(expected.shallow_eq(&res));

    let addr3 = runtime.deploy_contract(contract_src);
    let expected: Val = 30_i32.into_val(&runtime.env);
    let res = runtime.invoke_contract(&addr3, "subscript_sum", vec![]);
    assert!(expected.shallow_eq(&res));
}
