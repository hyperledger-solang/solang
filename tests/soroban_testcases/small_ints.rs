// SPDX-License-Identifier: Apache-2.0

//! Integers narrower than a Soroban width (uint8, int40, ...) keep their declared
//! width inside the contract, and cross the boundary as the next Soroban width.

use crate::build_solidity;
use soroban_sdk::{Bytes, Env, FromVal, IntoVal, Symbol, Val, Vec as SVec, U256};

fn reverts(env: &Env, addr: &soroban_sdk::Address, f: &str, args: SVec<Val>) -> bool {
    env.try_invoke_contract::<Val, Val>(addr, &Symbol::new(env, f), args)
        .is_err()
}

#[test]
fn small_int_arithmetic() {
    let runtime = build_solidity(
        r#"contract C {
            function add8(uint8 a, uint8 b) public pure returns (uint8) { return a + b; }
            function addi8(int8 a, int8 b) public pure returns (int8) { return a + b; }
            function shl(uint8 a) public pure returns (uint8) { return a << 4; }
            function not8(uint8 a) public pure returns (uint8) { return ~a; }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    // uint8 travels as a U32, but overflows at 8 bits.
    let res = runtime.invoke_contract(
        addr,
        "add8",
        vec![200_u32.into_val(env), 50_u32.into_val(env)],
    );
    assert_eq!(u32::from_val(env, &res), 250);
    assert!(reverts(
        env,
        addr,
        "add8",
        soroban_sdk::vec![env, 200_u32.into_val(env), 100_u32.into_val(env)]
    ));

    let res = runtime.invoke_contract(
        addr,
        "addi8",
        vec![(-100_i32).into_val(env), (-28_i32).into_val(env)],
    );
    assert_eq!(i32::from_val(env, &res), -128);
    assert!(reverts(
        env,
        addr,
        "addi8",
        soroban_sdk::vec![env, (-100_i32).into_val(env), (-29_i32).into_val(env)]
    ));

    // Shifts and `~` work on 8 bits.
    let res = runtime.invoke_contract(addr, "shl", vec![0x81_u32.into_val(env)]);
    assert_eq!(u32::from_val(env, &res), 0x10);
    let res = runtime.invoke_contract(addr, "not8", vec![0_u32.into_val(env)]);
    assert_eq!(u32::from_val(env, &res), 0xff);
}

#[test]
fn small_int_input_out_of_range() {
    let runtime = build_solidity(
        r#"contract C {
            function u(uint8 a) public pure returns (uint8) { return a; }
            function i(int8 a) public pure returns (int8) { return a; }
            function u40(uint40 a) public pure returns (uint40) { return a; }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "u", vec![255_u32.into_val(env)]);
    assert_eq!(u32::from_val(env, &res), 255);
    assert!(reverts(
        env,
        addr,
        "u",
        soroban_sdk::vec![env, 256_u32.into_val(env)]
    ));

    let res = runtime.invoke_contract(addr, "i", vec![(-128_i32).into_val(env)]);
    assert_eq!(i32::from_val(env, &res), -128);
    let res = runtime.invoke_contract(addr, "i", vec![127_i32.into_val(env)]);
    assert_eq!(i32::from_val(env, &res), 127);
    assert!(reverts(
        env,
        addr,
        "i",
        soroban_sdk::vec![env, (-129_i32).into_val(env)]
    ));
    assert!(reverts(
        env,
        addr,
        "i",
        soroban_sdk::vec![env, 128_i32.into_val(env)]
    ));

    // An odd width travels as the next Soroban width: uint40 is a U64.
    let max40 = (1_u64 << 40) - 1;
    let res = runtime.invoke_contract(addr, "u40", vec![max40.into_val(env)]);
    assert_eq!(u64::from_val(env, &res), max40);
    assert!(reverts(
        env,
        addr,
        "u40",
        soroban_sdk::vec![env, (1_u64 << 40).into_val(env)]
    ));
}

#[test]
fn small_int_bytes_conversion() {
    let runtime = build_solidity(
        r#"contract C {
            function b(uint8 a) public pure returns (bytes1) { return bytes1(a); }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let res = runtime.invoke_contract(addr, "b", vec![65_u32.into_val(env)]);
    assert_eq!(Bytes::from_val(env, &res), Bytes::from_slice(env, b"A"));
}

#[test]
fn small_int_storage_and_arrays() {
    let runtime = build_solidity(
        r#"contract C {
            uint8 x;
            function set(uint8 v) public { x = v; }
            function get() public view returns (uint8) { return x; }
            function bump(uint8 v) public { x += v; }
            function arr(uint8[] memory a) public pure returns (uint8[] memory) { return a; }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    runtime.invoke_contract(addr, "set", vec![200_u32.into_val(env)]);
    let res = runtime.invoke_contract(addr, "get", vec![]);
    assert_eq!(u32::from_val(env, &res), 200);
    assert!(reverts(
        env,
        addr,
        "bump",
        soroban_sdk::vec![env, 100_u32.into_val(env)]
    ));

    let a: SVec<u32> = soroban_sdk::vec![env, 1, 255];
    let res = runtime.invoke_contract(addr, "arr", vec![a.clone().into_val(env)]);
    assert_eq!(SVec::<u32>::from_val(env, &res), a);
    let bad: SVec<u32> = soroban_sdk::vec![env, 1, 256];
    assert!(reverts(
        env,
        addr,
        "arr",
        soroban_sdk::vec![env, bad.into_val(env)]
    ));
}

#[test]
fn odd_widths_travel_as_next_soroban_width() {
    let runtime = build_solidity(
        r#"contract C {
            function i56(int56 a) public pure returns (int56) { return a; }
            function u96(uint96 a) public pure returns (uint96) { return a; }
            function i96(int96 a) public pure returns (int96) { return a; }
            function u200(uint200 a) public pure returns (uint200) { return a; }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    // int56 is an I64: [-2^55, 2^55 - 1]
    for v in [-(1_i64 << 55), (1_i64 << 55) - 1] {
        let res = runtime.invoke_contract(addr, "i56", vec![v.into_val(env)]);
        assert_eq!(i64::from_val(env, &res), v);
    }
    assert!(reverts(
        env,
        addr,
        "i56",
        soroban_sdk::vec![env, (1_i64 << 55).into_val(env)]
    ));
    assert!(reverts(
        env,
        addr,
        "i56",
        soroban_sdk::vec![env, (-(1_i64 << 55) - 1).into_val(env)]
    ));

    // uint96 is a U128: [0, 2^96 - 1]
    let max96 = (1_u128 << 96) - 1;
    let res = runtime.invoke_contract(addr, "u96", vec![max96.into_val(env)]);
    assert_eq!(u128::from_val(env, &res), max96);
    assert!(reverts(
        env,
        addr,
        "u96",
        soroban_sdk::vec![env, (1_u128 << 96).into_val(env)]
    ));

    // int96 is an I128: [-2^95, 2^95 - 1]
    let min96 = -(1_i128 << 95);
    let res = runtime.invoke_contract(addr, "i96", vec![min96.into_val(env)]);
    assert_eq!(i128::from_val(env, &res), min96);
    assert!(reverts(
        env,
        addr,
        "i96",
        soroban_sdk::vec![env, (min96 - 1).into_val(env)]
    ));

    // uint200 is a U256: [0, 2^200 - 1]
    let max200 = U256::from_u32(env, 1).shl(200).sub(&U256::from_u32(env, 1));
    let res = runtime.invoke_contract(addr, "u200", vec![max200.clone().into_val(env)]);
    assert_eq!(U256::from_val(env, &res), max200);
    let too_big = U256::from_u32(env, 1).shl(200);
    assert!(reverts(
        env,
        addr,
        "u200",
        soroban_sdk::vec![env, too_big.into_val(env)]
    ));
}
