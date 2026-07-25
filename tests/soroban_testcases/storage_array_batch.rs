// SPDX-License-Identifier: Apache-2.0

//! Tests for the "arrays in contract storage" batch: one operation per type,
//! exercising the codegen path in `src/codegen/targets/soroban/arrays.rs`.
//! Kept separate from the pre-existing `storage_array.rs` cases.

use crate::build_solidity;
use soroban_sdk::{IntoVal, Val};

// Phase 1 (push): basic i32 scalar storage array. Push a few elements and read
// them back via subscript to confirm the codegen push path (arrays.rs).
#[test]
fn storage_array_push_i32_test() {
    let contract_src = r#"
        contract storage_array_i32 {
            int32[] mylist;

            function push_sum() public returns (int32) {
                mylist.push(5);
                mylist.push(10);
                mylist.push(15);
                return mylist[0] + mylist[1] + mylist[2]; // 30
            }
        }
    "#;

    let runtime = build_solidity(contract_src, |_| {});

    let addr = runtime.contracts.last().unwrap();
    let expected: Val = 30_i32.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "push_sum", vec![]);
    assert!(expected.shallow_eq(&res));
}
