// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use indexmap::Equivalent;
use soroban_sdk::{testutils::Address as _, Address, FromVal, IntoVal, Val};

// TODO: TESTS
// nested struct
// array as a struct member
// enum as a struct member

macro_rules! contract_single_field {
    ($t:expr) => {
        format!(
            "
                contract c {{
                    struct S {{ {} v; }}
                    S s;
                    function set({} val) public {{ s.v = val; }}
                    function get() public view returns ({}) {{ return s.v; }}
                }}
            ",
            $t, $t, $t
        )
    };
}

#[test]
fn struct_single_field_bool() {
    let solidity_contract = contract_single_field!("bool");
    let runtime = build_solidity(&solidity_contract, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let false_val: Val = false.into_val(&runtime.env);
    assert!(false_val.shallow_eq(&runtime.invoke_contract(addr, "get", vec![])));
    runtime.invoke_contract(addr, "set", vec![true.into_val(&runtime.env)]);
    let v: bool = FromVal::from_val(&runtime.env, &runtime.invoke_contract(addr, "get", vec![]));
    assert!(v);
}

#[test]
fn struct_single_field_int32() {
    let solidity_contract = contract_single_field!("int32");
    let runtime = build_solidity(&solidity_contract, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let zero: Val = 0_i32.into_val(&runtime.env);
    assert!(zero.shallow_eq(&runtime.invoke_contract(addr, "get", vec![])));
    runtime.invoke_contract(addr, "set", vec![5_i32.into_val(&runtime.env)]);
    let v: i32 = FromVal::from_val(&runtime.env, &runtime.invoke_contract(addr, "get", vec![]));
    assert_eq!(v, 5);
}

#[test]
fn struct_single_field_uint32() {
    let solidity_contract = contract_single_field!("uint32");
    let runtime = build_solidity(&solidity_contract, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let zero: Val = 0_u32.into_val(&runtime.env);
    assert!(zero.shallow_eq(&runtime.invoke_contract(addr, "get", vec![])));
    runtime.invoke_contract(addr, "set", vec![7_u32.into_val(&runtime.env)]);
    let v: u32 = FromVal::from_val(&runtime.env, &runtime.invoke_contract(addr, "get", vec![]));
    assert_eq!(v, 7);
}

#[test]
fn struct_single_field_int64() {
    let solidity_contract = contract_single_field!("int64");
    let runtime = build_solidity(&solidity_contract, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let zero: Val = 0_i64.into_val(&runtime.env);
    assert!(zero.shallow_eq(&runtime.invoke_contract(addr, "get", vec![])));

    runtime.invoke_contract(addr, "set", vec![10_i64.into_val(&runtime.env)]);
    let v: i64 = FromVal::from_val(&runtime.env, &runtime.invoke_contract(addr, "get", vec![]));
    assert_eq!(v, 10);
}

#[test]
fn struct_single_field_uint64() {
    let solidity_contract = contract_single_field!("uint64");
    let runtime = build_solidity(&solidity_contract, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let zero: Val = 0_u64.into_val(&runtime.env);
    assert!(zero.shallow_eq(&runtime.invoke_contract(addr, "get", vec![])));
    runtime.invoke_contract(addr, "set", vec![42_u64.into_val(&runtime.env)]);
    let v: u64 = FromVal::from_val(&runtime.env, &runtime.invoke_contract(addr, "get", vec![]));
    assert_eq!(v, 42);
}

#[test]
fn struct_single_field_int128() {
    let solidity_contract = contract_single_field!("int128");
    let runtime = build_solidity(&solidity_contract, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let zero: Val = 0_i128.into_val(&runtime.env);
    assert!(zero.shallow_eq(&runtime.invoke_contract(addr, "get", vec![])));
    runtime.invoke_contract(addr, "set", vec![100_i128.into_val(&runtime.env)]);
    let v: i128 = FromVal::from_val(&runtime.env, &runtime.invoke_contract(addr, "get", vec![]));
    assert_eq!(v, 100);
}

#[test]
fn struct_single_field_string() {
    let solidity_contract = contract_single_field!("string");
    let runtime = build_solidity(&solidity_contract, |_| {});
    let addr = runtime.contracts.last().unwrap();

    let input = soroban_sdk::String::from_str(&runtime.env, "hello").into_val(&runtime.env);
    runtime.invoke_contract(addr, "set", vec![input]);
    let result = runtime.invoke_contract(addr, "get", vec![]);
    let got = soroban_sdk::String::from_val(&runtime.env, &result);
    assert_eq!(got, soroban_sdk::String::from_str(&runtime.env, "hello"));
}

#[test]
fn struct_single_field_bytes() {
    let solidity_contract = contract_single_field!("bytes");
    let runtime = build_solidity(&solidity_contract, |_| {});
    let addr = runtime.contracts.last().unwrap();

    let payload: [u8; 3] = [0x01, 0x02, 0x03];
    let input = soroban_sdk::Bytes::from_array(&runtime.env, &payload).into_val(&runtime.env);
    runtime.invoke_contract(addr, "set", vec![input]);
    let result = runtime.invoke_contract(addr, "get", vec![]);
    let got = soroban_sdk::Bytes::from_val(&runtime.env, &result);
    assert_eq!(got, soroban_sdk::Bytes::from_slice(&runtime.env, &payload));
}

#[test]
fn struct_single_field_bytes4() {
    let solidity_contract = contract_single_field!("bytes4");
    let runtime = build_solidity(&solidity_contract, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let payload: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xDD];
    let input: Val = soroban_sdk::BytesN::from_array(&runtime.env, &payload).into_val(&runtime.env);
    runtime.invoke_contract(addr, "set", vec![input]);
    let result = runtime.invoke_contract(addr, "get", vec![]);
    let got: soroban_sdk::BytesN<4> = soroban_sdk::BytesN::from_val(&runtime.env, &result);
    assert_eq!(got, soroban_sdk::BytesN::from_array(&runtime.env, &payload));
}

#[test]
fn struct_single_field_address() {
    let solidity_contract = contract_single_field!("address");
    let runtime = build_solidity(&solidity_contract, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let who = Address::generate(&runtime.env);
    runtime.invoke_contract(addr, "set", vec![who.clone().into_val(&runtime.env)]);
    let result = runtime.invoke_contract(addr, "get", vec![]);
    let got = Address::from_val(&runtime.env, &result);
    assert!(got.equivalent(&who));
}
