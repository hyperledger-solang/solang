// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{FromVal, IntoVal, Val};

#[test]
fn abi_encode_bool_roundtrip() {
    let runtime = build_solidity(
        r#"contract T {
            function f(bool x) public pure returns (bool) {
                bytes memory enc = abi.encode(x);
                return abi.decode(enc, (bool));
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    for v in [false, true] {
        let res = runtime.invoke_contract(addr, "f", vec![v.into_val(&runtime.env)]);
        assert_eq!(
            bool::from_val(&runtime.env, &res),
            v,
            "bool roundtrip failed for {v}"
        );
    }
}

#[test]
fn abi_encode_uint32_roundtrip() {
    let runtime = build_solidity(
        r#"contract T {
            function f(uint32 x) public pure returns (uint32) {
                bytes memory enc = abi.encode(x);
                return abi.decode(enc, (uint32));
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    for v in [0_u32, 1, 42, u32::MAX] {
        let res = runtime.invoke_contract(addr, "f", vec![v.into_val(&runtime.env)]);
        assert_eq!(
            u32::from_val(&runtime.env, &res),
            v,
            "uint32 roundtrip failed for {v}"
        );
    }
}

#[test]
fn abi_encode_int32_roundtrip() {
    let runtime = build_solidity(
        r#"contract T {
            function f(int32 x) public pure returns (int32) {
                bytes memory enc = abi.encode(x);
                return abi.decode(enc, (int32));
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    for v in [0_i32, 1, -1, 42, -42, i32::MAX, i32::MIN] {
        let res = runtime.invoke_contract(addr, "f", vec![v.into_val(&runtime.env)]);
        assert_eq!(
            i32::from_val(&runtime.env, &res),
            v,
            "int32 roundtrip failed for {v}"
        );
    }
}

#[test]
fn abi_encode_uint64_roundtrip() {
    let runtime = build_solidity(
        r#"contract T {
            function f(uint64 x) public pure returns (uint64) {
                bytes memory enc = abi.encode(x);
                return abi.decode(enc, (uint64));
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    for v in [0_u64, 1, (1 << 56) - 1, 1 << 56, u64::MAX] {
        let res = runtime.invoke_contract(addr, "f", vec![v.into_val(&runtime.env)]);
        assert_eq!(
            u64::from_val(&runtime.env, &res),
            v,
            "uint64 roundtrip failed for {v}"
        );
    }
}

#[test]
fn abi_encode_int64_roundtrip() {
    let runtime = build_solidity(
        r#"contract T {
            function f(int64 x) public pure returns (int64) {
                bytes memory enc = abi.encode(x);
                return abi.decode(enc, (int64));
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    for v in [0_i64, 1, -1, 12345, -12345, i64::MAX, i64::MIN] {
        let res = runtime.invoke_contract(addr, "f", vec![v.into_val(&runtime.env)]);
        assert_eq!(
            i64::from_val(&runtime.env, &res),
            v,
            "int64 roundtrip failed for {v}"
        );
    }
}

#[test]
fn abi_encode_uint128_roundtrip() {
    let runtime = build_solidity(
        r#"contract T {
            function f(uint128 x) public pure returns (uint128) {
                bytes memory enc = abi.encode(x);
                return abi.decode(enc, (uint128));
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    for v in [0_u128, 1, u64::MAX as u128, (1u128 << 64) + 5, u128::MAX] {
        let res = runtime.invoke_contract(addr, "f", vec![v.into_val(&runtime.env)]);
        assert_eq!(
            u128::from_val(&runtime.env, &res),
            v,
            "uint128 roundtrip failed for {v}"
        );
    }
}

#[test]
fn abi_encode_int128_roundtrip() {
    let runtime = build_solidity(
        r#"contract T {
            function f(int128 x) public pure returns (int128) {
                bytes memory enc = abi.encode(x);
                return abi.decode(enc, (int128));
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    for v in [0_i128, 1, -1, 999, -999, i128::MAX, i128::MIN] {
        let res = runtime.invoke_contract(addr, "f", vec![v.into_val(&runtime.env)]);
        assert_eq!(
            i128::from_val(&runtime.env, &res),
            v,
            "int128 roundtrip failed for {v}"
        );
    }
}

#[test]
fn abi_encode_string_roundtrip() {
    let runtime = build_solidity(
        r#"contract T {
            function f(string memory x) public pure returns (string memory) {
                bytes memory enc = abi.encode(x);
                return abi.decode(enc, (string));
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    for s in ["hello", "world", "paused"] {
        let arg: Val = soroban_sdk::String::from_str(&runtime.env, s).into_val(&runtime.env);
        let res = runtime.invoke_contract(addr, "f", vec![arg]);
        let got = soroban_sdk::String::from_val(&runtime.env, &res);
        assert_eq!(
            got,
            soroban_sdk::String::from_str(&runtime.env, s),
            "string roundtrip failed for {s:?}"
        );
    }
}

#[test]
fn abi_encode_bytes_roundtrip() {
    let runtime = build_solidity(
        r#"contract T {
            function f(bytes memory x) public pure returns (bytes memory) {
                bytes memory enc = abi.encode(x);
                return abi.decode(enc, (bytes));
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    for data in [[0xDE, 0xAD, 0xBE, 0xEF].as_slice(), &[0x00], &[0xFF; 32]] {
        let arg: Val = soroban_sdk::Bytes::from_slice(&runtime.env, data).into_val(&runtime.env);
        let res = runtime.invoke_contract(addr, "f", vec![arg]);
        let got = soroban_sdk::Bytes::from_val(&runtime.env, &res);
        assert_eq!(
            got,
            soroban_sdk::Bytes::from_slice(&runtime.env, data),
            "bytes roundtrip failed for {data:?}"
        );
    }
}

#[test]
fn abi_encode_allocates_8_bytes_per_arg() {
    let runtime = build_solidity(
        r#"contract T {
            function f() public pure returns (uint32) {
                bytes memory enc = abi.encode("paused");
                return enc.length;
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let res = runtime.invoke_contract(addr, "f", vec![]);
    let len: u32 = FromVal::from_val(&runtime.env, &res);
    assert_eq!(len, 8, "expected 8 bytes per encoded argument");
}
