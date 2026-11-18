// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{FromVal, IntoVal, Symbol, Val};

#[test]
fn enum_out_of_range_reverts() {
    let runtime = build_solidity(
        r#"contract C {
            enum E { A, B, C }
            function f(E e) public pure returns (uint32) { return uint32(e); }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "f", vec![2_u32.into_val(env)]);
    assert_eq!(u32::from_val(env, &res), 2);

    let args = soroban_sdk::vec![env, 3_u32.into_val(env)];
    let res = env.try_invoke_contract::<Val, Val>(addr, &Symbol::new(env, "f"), args);
    assert!(res.is_err(), "f(3) must revert: 3 is not a member of E");
}

#[test]
fn bool_must_be_true_or_false() {
    let runtime = build_solidity(
        r#"contract C {
            function f(bool b) public pure returns (bool) { return !b; }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "f", vec![true.into_val(env)]);
    assert!(!bool::from_val(env, &res));
    let res = runtime.invoke_contract(addr, "f", vec![false.into_val(env)]);
    assert!(bool::from_val(env, &res));

    let args = soroban_sdk::vec![env, 1_u32.into_val(env)];
    let res = env.try_invoke_contract::<Val, Val>(addr, &Symbol::new(env, "f"), args);
    assert!(res.is_err(), "a u32 is not a bool, f must revert");
}

#[test]
fn shift_by_full_width() {
    let runtime = build_solidity(
        r#"contract C {
            function l(uint64 a, uint64 b) public pure returns (uint64) { return a << b; }
            function ur(uint64 a, uint64 b) public pure returns (uint64) { return a >> b; }
            function r(int64 a, uint64 b) public pure returns (int64) { return a >> b; }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;
    let call = |f: &str, a: Val, b: u64| runtime.invoke_contract(addr, f, vec![a, b.into_val(env)]);

    assert_eq!(u64::from_val(env, &call("l", 3_u64.into_val(env), 1)), 6);
    assert_eq!(u64::from_val(env, &call("l", 3_u64.into_val(env), 64)), 0);
    assert_eq!(u64::from_val(env, &call("ur", 3_u64.into_val(env), 100)), 0);
    assert_eq!(
        i64::from_val(env, &call("r", (-5_i64).into_val(env), 64)),
        -1
    );
    assert_eq!(i64::from_val(env, &call("r", 5_i64.into_val(env), 64)), 0);
}
