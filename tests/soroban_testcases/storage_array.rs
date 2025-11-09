// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{IntoVal, Val};

#[test]
fn storage_array_ops_test() {
    let contract_src = r#"
        contract storage_array {
            uint64[] mylist;
            uint64 normal = 20;

            function push_pop() public returns (uint64) {
                mylist.push(5);
                //mylist.push(10);
                //mylist.pop();

                //uint64 len = uint64(mylist.length);

                mylist[0] = 15;

                mylist.push(5);

                return mylist[0] + mylist[1];
            }

            function loop() public returns (uint64) {
                uint64 sum = 0;

                mylist.push(5);
                mylist.push(10);
                mylist.push(15);

                for (uint64 i = 0; i < mylist.length; i++) {
                    sum += mylist[i];
                }

                return sum;
            }

            function random_access(uint64 index) public returns (uint64) {
                uint64 sum = 0;

                mylist.push(5);
                mylist.push(10);
                mylist.push(15);

                sum += mylist[index];
                sum += mylist[index + 1];

                return sum;
            }

            function pop_len() public returns (uint64) {
                mylist.push(1);
                mylist.push(2);
                mylist.push(3);

                mylist.pop();
                mylist.pop();

                return mylist.length;
            }

            // Copy a memory array into storage using push
            function mem_to_storage() public returns (uint64) {
                uint64[] memory tmp = new uint64[](3);
                tmp[0] = 1;
                tmp[1] = 2;
                tmp[2] = 3;

                for (uint64 i = 0; i < tmp.length; i++) {
                    mylist.push(tmp[i]);
                }

                uint64 sum = 0;
                for (uint64 i = 0; i < mylist.length; i++) {
                    sum += mylist[i];
                }
                return sum; // 1+2+3 = 6
            }

            // Copy a storage array into memory and sum
            function storage_to_mem() public returns (uint64) {
                mylist.push(7);
                mylist.push(9);
                mylist.push(11);

                uint64[] memory tmp = new uint64[](mylist.length);
                for (uint64 i = 0; i < mylist.length; i++) {
                    tmp[i] = mylist[i];
                }

                uint64 sum = 0;
                for (uint64 i = 0; i < tmp.length; i++) {
                    sum += tmp[i];
                }
                return sum; // 7+9+11 = 27
            }
        }
    "#;

    // Build once; deploy fresh instances for each scenario to avoid state carryover.
    let mut runtime = build_solidity(contract_src, |_| {});

    // 1) push_pop(): after operations -> [15, 5]; return 15 + 5 = 20
    let addr = runtime.contracts.last().unwrap();
    let expected: Val = 20_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "push_pop", vec![]);
    assert!(expected.shallow_eq(&res));

    // 2) loop(): new instance, pushes 5,10,15 and sums => 30
    let addr2 = runtime.deploy_contract(contract_src);
    let expected: Val = 30_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(&addr2, "loop", vec![]);
    assert!(expected.shallow_eq(&res));

    // 3) random_access(index): new instance
    let addr3 = runtime.deploy_contract(contract_src);

    // index 0: 5 + 10 = 15
    let expected: Val = 15_u64.into_val(&runtime.env);
    let args = vec![0_u64.into_val(&runtime.env)];
    let res = runtime.invoke_contract(&addr3, "random_access", args);
    assert!(expected.shallow_eq(&res));

    // index 1: 10 + 15 = 25
    let expected: Val = 25_u64.into_val(&runtime.env);
    let args = vec![1_u64.into_val(&runtime.env)];
    let res = runtime.invoke_contract(&addr3, "random_access", args);
    assert!(expected.shallow_eq(&res));

    // 4) pop_len(): start with [], push 3 items then pop 2 => length = 1
    let addr4 = runtime.deploy_contract(contract_src);
    let expected: Val = 1_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(&addr4, "pop_len", vec![]);
    assert!(expected.shallow_eq(&res));

    // 5) mem_to_storage(): copy [1,2,3] into storage and sum => 6
    let addr5 = runtime.deploy_contract(contract_src);
    let expected: Val = 6_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(&addr5, "mem_to_storage", vec![]);
    assert!(expected.shallow_eq(&res));

    // 6) storage_to_mem(): start storage [7,9,11], copy to memory and sum => 27
    let addr6 = runtime.deploy_contract(contract_src);
    let expected: Val = 27_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(&addr6, "storage_to_mem", vec![]);
    assert!(expected.shallow_eq(&res));
}
