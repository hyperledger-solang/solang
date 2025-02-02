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

    let mut runtime = SorobanEnv::new();
    // No constructor arguments
    let constructor_args: soroban_sdk::Vec<Val> = soroban_sdk::Vec::new(&runtime.env);
    let addr = runtime.register_contract(wasm, constructor_args);

    let res = runtime.invoke_contract(&addr, "count", vec![]);
    let expected: Val = 10_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    runtime.invoke_contract(&addr, "increment", vec![]);
    let res = runtime.invoke_contract(&addr, "count", vec![]);
    let expected: Val = 11_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    runtime.invoke_contract(&addr, "decrement", vec![]);
    let res = runtime.invoke_contract(&addr, "count", vec![]);
    let expected: Val = 10_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));
}

#[test]
fn different_storage_types() {
    let wasm = build_solidity(
        r#"contract sesa {
            
    uint64 public temporary sesa = 1;
    uint64 public instance sesa1 = 1;
    uint64 public persistent sesa2 = 2;
    uint64 public sesa3 = 2;

    function inc() public {
        sesa++;
        sesa1++;
        sesa2++;
        sesa3++;
    }

    function dec() public {
        sesa--;
        sesa1--;
        sesa2--;
        sesa3--;
    }
}"#,
    );

    let mut runtime = SorobanEnv::new();
    // No constructor arguments
    let constructor_args: soroban_sdk::Vec<Val> = soroban_sdk::Vec::new(&runtime.env);
    let addr = runtime.register_contract(wasm, constructor_args);

    let res = runtime.invoke_contract(&addr, "sesa", vec![]);
    let expected: Val = 1_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(&addr, "sesa1", vec![]);
    let expected: Val = 1_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(&addr, "sesa2", vec![]);
    let expected: Val = 2_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(&addr, "sesa3", vec![]);
    let expected: Val = 2_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    runtime.invoke_contract(&addr, "inc", vec![]);
    let res = runtime.invoke_contract(&addr, "sesa", vec![]);
    let expected: Val = 2_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(&addr, "sesa1", vec![]);
    let expected: Val = 2_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(&addr, "sesa2", vec![]);
    let expected: Val = 3_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(&addr, "sesa3", vec![]);
    let expected: Val = 3_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    runtime.invoke_contract(&addr, "dec", vec![]);
    let res = runtime.invoke_contract(&addr, "sesa", vec![]);
    let expected: Val = 1_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(&addr, "sesa1", vec![]);
    let expected: Val = 1_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(&addr, "sesa2", vec![]);
    let expected: Val = 2_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    let res = runtime.invoke_contract(&addr, "sesa3", vec![]);
    let expected: Val = 2_u64.into_val(&runtime.env);
    assert!(expected.shallow_eq(&res));

    // FIXME: we need to figure out how to capture the compiler diagnostics but also
    //        allow the test itself to define its own SorobanEnv (needed for ttl tests)
    // let diags = runtime.compiler_diagnostics;

    // assert!(diags
    //     .contains_message("storage type not specified for `sesa3`, defaulting to `persistent`"));
}
