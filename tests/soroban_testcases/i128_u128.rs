use crate::build_solidity;
use soroban_sdk::{FromVal, IntoVal, Val};

#[test]
fn uint128_high_limb_not_dropped_on_encode() {
    let runtime = build_solidity(
        r#"contract test {
            function id(uint128 a) public returns (uint128) { return a; }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let value: u128 = (1u128 << 64) + 5; // high limb set, low limb < 2**56
    let arg: Val = value.into_val(&runtime.env);
    let res: Val = runtime.invoke_contract(addr, "id", vec![arg]);
    let got: u128 = FromVal::from_val(&runtime.env, &res);
    assert_eq!(got, value, "uint128 high 64 bits were dropped on encode");
}

#[test]
fn i128_u128_encode_decode_coverage() {
    let runtime = build_solidity(
        r#"contract test {
            function id_u(uint128 a) public returns (uint128) { return a; }
            function id_i(int128 a) public returns (int128) { return a; }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();

    // uint128 test values: zero, small (56-bit), boundary, high bits, edge cases
    let u_vals = [
        0u128,
        1,
        (1 << 56) - 1,
        1 << 56,
        (1 << 64) - 1,
        1 << 64,
        (1 << 64) + 5,
        u64::MAX as u128,
        u128::MAX,
    ];

    for &v in &u_vals {
        let res = runtime.invoke_contract(addr, "id_u", vec![v.into_val(&runtime.env)]);
        assert_eq!(
            u128::from_val(&runtime.env, &res),
            v,
            "uint128 failed for {}",
            v
        );
    }

    // int128 test values: zero, small pos/neg (56-bit), boundary, high bits, edge cases
    let i_vals = [
        0i128,
        1,
        (1 << 55) - 1,
        1 << 55,
        (1 << 63) - 1,
        -(1 << 55),
        -(1 << 55) - 1,
        -1,
        i64::MIN as i128,
        i128::MIN,
        i128::MAX,
    ];

    for &v in &i_vals {
        let res = runtime.invoke_contract(addr, "id_i", vec![v.into_val(&runtime.env)]);
        assert_eq!(
            i128::from_val(&runtime.env, &res),
            v,
            "int128 failed for {}",
            v
        );
    }
}
