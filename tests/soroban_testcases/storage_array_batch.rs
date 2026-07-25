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

#[test]
fn storage_array_subscript_read_i32_test() {
    let contract_src = r#"
        contract storage_array_read_i32 {
            int32[] mylist;

            function read_at(uint32 i) public returns (int32) {
                mylist.push(5);
                mylist.push(10);
                mylist.push(15);
                return mylist[i];
            }
        }
    "#;

    let mut runtime = build_solidity(contract_src, |_| {});

    let addr = runtime.contracts.last().unwrap();
    let expected: Val = 5_i32.into_val(&runtime.env);
    let args = vec![0_u32.into_val(&runtime.env)];
    let res = runtime.invoke_contract(addr, "read_at", args);
    assert!(expected.shallow_eq(&res));

    let addr2 = runtime.deploy_contract(contract_src);
    let expected: Val = 15_i32.into_val(&runtime.env);
    let args = vec![2_u32.into_val(&runtime.env)];
    let res = runtime.invoke_contract(&addr2, "read_at", args);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn storage_array_subscript_write_i32_test() {
    let contract_src = r#"
        contract storage_array_write_i32 {
            int32[] mylist;

            function build_and_overwrite() public returns (int32) {
                mylist.push(5);
                mylist.push(10);
                mylist.push(15);
                mylist[1] = 100;
                return mylist[0] + mylist[1] + mylist[2];
            }

            function write_at(uint32 i, int32 v) public returns (int32) {
                mylist.push(1);
                mylist.push(2);
                mylist.push(3);
                mylist[i] = v;
                return mylist[i];
            }
        }
    "#;

    let mut runtime = build_solidity(contract_src, |_| {});

    let addr = runtime.contracts.last().unwrap();
    let expected: Val = 120_i32.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "build_and_overwrite", vec![]);
    assert!(expected.shallow_eq(&res));

    let addr2 = runtime.deploy_contract(contract_src);
    let expected: Val = 42_i32.into_val(&runtime.env);
    let args = vec![2_u32.into_val(&runtime.env), 42_i32.into_val(&runtime.env)];
    let res = runtime.invoke_contract(&addr2, "write_at", args);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn storage_array_bool_test() {
    let contract_src = r#"
        contract storage_array_bool {
            bool[] mylist;

            function push_read() public returns (bool) {
                mylist.push(true);
                mylist.push(false);
                mylist.push(true);
                return mylist[0] && !mylist[1] && mylist[2];
            }

            function len_after_push() public returns (uint32) {
                mylist.push(true);
                mylist.push(false);
                return uint32(mylist.length);
            }

            function write_read() public returns (bool) {
                mylist.push(false);
                mylist.push(false);
                mylist[1] = true;
                return mylist[1];
            }

            function pop_len() public returns (uint32) {
                mylist.push(true);
                mylist.push(false);
                mylist.push(true);
                mylist.pop();
                return uint32(mylist.length);
            }
        }
    "#;

    let mut runtime = build_solidity(contract_src, |_| {});

    let addr = runtime.contracts.last().unwrap();
    let expected: Val = true.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "push_read", vec![]);
    assert!(expected.shallow_eq(&res));

    let addr2 = runtime.deploy_contract(contract_src);
    let expected: Val = 2_u32.into_val(&runtime.env);
    let res = runtime.invoke_contract(&addr2, "len_after_push", vec![]);
    assert!(expected.shallow_eq(&res));

    let addr3 = runtime.deploy_contract(contract_src);
    let expected: Val = true.into_val(&runtime.env);
    let res = runtime.invoke_contract(&addr3, "write_read", vec![]);
    assert!(expected.shallow_eq(&res));

    let addr4 = runtime.deploy_contract(contract_src);
    let expected: Val = 2_u32.into_val(&runtime.env);
    let res = runtime.invoke_contract(&addr4, "pop_len", vec![]);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn storage_array_i32_test() {
    let contract_src = r#"
        contract storage_array_i32_full {
            int32[] mylist;

            // Build [-5,-3,-1,1,3,5], mutate via subscripts, pop twice, loop-sum.
            function churn() public returns (int32) {
                for (int32 i = 0; i < 6; i++) {
                    mylist.push(i * 2 - 5); // -5,-3,-1,1,3,5
                }
                // read-driven writes (binary ops)
                mylist[0] = mylist[5] + mylist[1]; // 5 + (-3) = 2 -> [2,-3,-1,1,3,5]
                mylist[4] = mylist[0] * mylist[3]; // 2 * 1   = 2 -> [2,-3,-1,1,2,5]

                mylist.pop(); // drop 5 -> [2,-3,-1,1,2]
                mylist.pop(); // drop 2 -> [2,-3,-1,1], length 4

                int32 sum = 0;
                for (uint32 j = 0; j < mylist.length; j++) {
                    sum += mylist[j]; // 2 + (-3) + (-1) + 1 = -1
                }
                return sum * 100 + int32(uint32(mylist.length)); // -1*100 + 4 = -96
            }

            // Push 1..n, swap first and last via a temp, return their difference.
            function swap_ends(uint32 n) public returns (int32) {
                for (uint32 k = 0; k < n; k++) {
                    mylist.push(int32(k) + 1); // 1,2,...,n
                }
                uint32 last = uint32(mylist.length) - 1;
                int32 tmp = mylist[0];
                mylist[0] = mylist[last];
                mylist[last] = tmp;
                return mylist[0] - mylist[last]; // n - 1
            }
        }
    "#;

    let mut runtime = build_solidity(contract_src, |_| {});

    let addr = runtime.contracts.last().unwrap();
    let expected: Val = (-96_i32).into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "churn", vec![]);
    assert!(expected.shallow_eq(&res));

    let addr2 = runtime.deploy_contract(contract_src);
    let expected: Val = 3_i32.into_val(&runtime.env);
    let args = vec![4_u32.into_val(&runtime.env)];
    let res = runtime.invoke_contract(&addr2, "swap_ends", args);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn storage_array_u32_test() {
    let contract_src = r#"
        contract storage_array_u32_full {
            uint32[] mylist;

            function churn() public returns (uint32) {
                for (uint32 i = 0; i < 5; i++) {
                    mylist.push((i + 1) * 10); // 10,20,30,40,50
                }
                mylist[0] = mylist[4] + mylist[1]; // 50 + 20 = 70 -> [70,20,30,40,50]
                mylist[2] = mylist[2] * 3;         // 30 * 3  = 90 -> [70,20,90,40,50]

                mylist.pop(); // drop 50 -> [70,20,90,40], length 4

                uint32 sum = 0;
                for (uint32 j = 0; j < mylist.length; j++) {
                    sum += mylist[j]; // 70 + 20 + 90 + 40 = 220
                }
                return sum + uint32(mylist.length); // 220 + 4 = 224
            }

            function set_get(uint32 i, uint32 v) public returns (uint32) {
                for (uint32 k = 0; k < 4; k++) {
                    mylist.push(k); // 0,1,2,3
                }
                mylist[i] = v;
                return mylist[i] + uint32(mylist.length);
            }
        }
    "#;

    let mut runtime = build_solidity(contract_src, |_| {});

    let addr = runtime.contracts.last().unwrap();
    let expected: Val = 224_u32.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "churn", vec![]);
    assert!(expected.shallow_eq(&res));

    let addr2 = runtime.deploy_contract(contract_src);
    let expected: Val = 103_u32.into_val(&runtime.env);
    let args = vec![2_u32.into_val(&runtime.env), 99_u32.into_val(&runtime.env)];
    let res = runtime.invoke_contract(&addr2, "set_get", args);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn storage_array_i64_test() {
    let contract_src = r#"
        contract storage_array_i64_full {
            int64[] mylist;

            function churn() public returns (int64) {
                mylist.push(1_000_000_000_000); // 1e12
                mylist.push(-500_000_000_000);  // -5e11
                mylist.push(3);
                mylist.push(-7);
                // [1e12, -5e11, 3, -7]
                mylist[2] = mylist[0] + mylist[1]; // 1e12 + (-5e11) = 5e11
                mylist[3] = mylist[3] * 100;       // -7 * 100 = -700
                // [1e12, -5e11, 5e11, -700]

                mylist.pop(); // drop -700 -> [1e12, -5e11, 5e11], length 3

                int64 sum = 0;
                for (uint32 j = 0; j < mylist.length; j++) {
                    sum += mylist[j]; // 1e12 - 5e11 + 5e11 = 1e12
                }
                return sum + int64(uint32(mylist.length)); // 1e12 + 3
            }

            function set_get(uint32 i, int64 v) public returns (int64) {
                mylist.push(10);
                mylist.push(20);
                mylist.push(30);
                mylist[i] = v;
                return mylist[i];
            }
        }
    "#;

    let mut runtime = build_solidity(contract_src, |_| {});

    let addr = runtime.contracts.last().unwrap();
    let expected: Val = 1_000_000_000_003_i64.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "churn", vec![]);
    assert!(expected.shallow_eq(&res));

    let addr2 = runtime.deploy_contract(contract_src);
    let expected: Val = (-999_i64).into_val(&runtime.env);
    let args = vec![
        1_u32.into_val(&runtime.env),
        (-999_i64).into_val(&runtime.env),
    ];
    let res = runtime.invoke_contract(&addr2, "set_get", args);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn storage_array_u64_test() {
    let contract_src = r#"
        contract storage_array_u64_full {
            uint64[] mylist;

            function churn() public returns (uint64) {
                mylist.push(10_000_000_000); // 1e10
                mylist.push(20_000_000_000); // 2e10
                mylist.push(5);
                mylist.push(7);
                // [1e10, 2e10, 5, 7]
                mylist[2] = mylist[0] + mylist[1]; // 3e10
                mylist[3] = mylist[1] * 2;         // 4e10
                // [1e10, 2e10, 3e10, 4e10]

                mylist.pop(); // drop 4e10 -> [1e10, 2e10, 3e10], length 3

                uint64 sum = 0;
                for (uint32 j = 0; j < mylist.length; j++) {
                    sum += mylist[j]; // 1e10 + 2e10 + 3e10 = 6e10
                }
                return sum + uint64(mylist.length); // 6e10 + 3
            }

            function set_get(uint32 i, uint64 v) public returns (uint64) {
                mylist.push(100);
                mylist.push(200);
                mylist.push(300);
                mylist[i] = v;
                return mylist[i];
            }
        }
    "#;

    let mut runtime = build_solidity(contract_src, |_| {});

    let addr = runtime.contracts.last().unwrap();
    let expected: Val = 60_000_000_003_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "churn", vec![]);
    assert!(expected.shallow_eq(&res));

    let addr2 = runtime.deploy_contract(contract_src);
    let expected: Val = 999_888_777_666_u64.into_val(&runtime.env);
    let args = vec![
        0_u32.into_val(&runtime.env),
        999_888_777_666_u64.into_val(&runtime.env),
    ];
    let res = runtime.invoke_contract(&addr2, "set_get", args);
    assert!(expected.shallow_eq(&res));
}
