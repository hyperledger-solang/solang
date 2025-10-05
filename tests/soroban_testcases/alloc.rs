// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{IntoVal, Val};

#[test]
fn arrays_basic_ops_test() {
    let runtime = build_solidity(
        r#"
        contract array {
            function push_pop() public returns (uint64) {
                uint64[] mylist;

                mylist.push(5);
                mylist.push(10);
                mylist.pop();

                uint64 len = mylist.length;

                return len + mylist[0];
            }

            function loop() public returns (uint64) {
                uint64[] mylist;
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
                uint64[] mylist;
                uint64 sum = 0;

                mylist.push(5);
                mylist.push(10);
                mylist.push(15);

                sum += mylist[index];
                sum += mylist[index + 1];

                return sum;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    // push_pop(): [5,10] -> pop -> [5]; len(=1) + mylist[0](=5) = 6
    let expected: Val = 6_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "push_pop", vec![]);
    println!("Result of push_pop: {:?}", res);
    assert!(expected.shallow_eq(&res));

    // loop(): 5 + 10 + 15 = 30
    let expected: Val = 30_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "loop", vec![]);
    assert!(expected.shallow_eq(&res));

    // random_access(0): mylist[0] + mylist[1] = 5 + 10 = 15
    let expected: Val = 15_u64.into_val(&runtime.env);
    let args = vec![0_u64.into_val(&runtime.env)];
    let res = runtime.invoke_contract(addr, "random_access", args);
    assert!(expected.shallow_eq(&res));

    // random_access(1): mylist[1] + mylist[2] = 10 + 15 = 25
    let expected: Val = 25_u64.into_val(&runtime.env);
    let args = vec![1_u64.into_val(&runtime.env)];
    let res = runtime.invoke_contract(addr, "random_access", args);
    assert!(expected.shallow_eq(&res));
}
