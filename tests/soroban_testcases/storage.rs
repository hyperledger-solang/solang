// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, SorobanEnv};
use soroban_sdk::{IntoVal, Val};

#[test]
fn counter() {
    let wasm = build_solidity(
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
    let mut src = SorobanEnv::new();
    // No constructor arguments
    let constructor_args: soroban_sdk::Vec<Val> = soroban_sdk::Vec::new(&src.env);
    let address = src.register_contract(wasm, constructor_args);

    let res = src.invoke_contract(&address, "count", vec![]);
    let expected: Val = 10_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&res));

    src.invoke_contract(&address, "increment", vec![]);
    let res = src.invoke_contract(&address, "count", vec![]);
    let expected: Val = 11_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&res));

    src.invoke_contract(&address, "decrement", vec![]);
    let res = src.invoke_contract(&address, "count", vec![]);
    let expected: Val = 10_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&res));
}
