// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{IntoVal, Val};

#[test]
fn counter() {
    let src = build_solidity(
        r#"contract counter {
            uint64 public count = 10;
        
            function increment() public returns (uint64){
                count += 1;
                return count;
            }
        
            function decrement() public returns (uint64){
                count -= 1;
                return count;
            }
        }"#,
    );

    let addr = src.contracts.last().unwrap();

    let _res = src.invoke_contract(addr, "init", vec![]);

    let res = src.invoke_contract(addr, "count", vec![]);
    let expected: Val = 10_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&res));

    src.invoke_contract(addr, "increment", vec![]);
    let res = src.invoke_contract(addr, "count", vec![]);
    let expected: Val = 11_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&res));

    src.invoke_contract(addr, "decrement", vec![]);
    let res = src.invoke_contract(addr, "count", vec![]);
    let expected: Val = 10_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&res));
}
