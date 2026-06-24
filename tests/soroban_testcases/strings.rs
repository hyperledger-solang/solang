// SPDX-License-Identifier: Apache-2.0

//! String and bytes tests for Soroban
//!
//! Test coverage:
//! - returns_string_literal: Returns a hardcoded string literal "hello"
//! - string_length: Gets the length of a string using bytes(s).length
//! - string_storage_set_get: Stores and retrieves a string from contract storage
//! - string_storage_overwrite: Overwrites a stored string with a new value
//! - bytes_length: Gets the length of bytes memory
//! - returns_string_from_var: Returns a string assigned to a local variable
//! - bytes_first_element: Accesses and returns the first element of bytes
//! - bytes_storage_round_trip: Stores and retrieves bytes from contract storage
//! - string_storage_read_before_write: Reads an uninitialized string storage (empty)
//! - bytes_memory_subscript_write: Writes a single byte at a specific index in memory
//! - bytes_memory_compound_or: Compound OR assignment (|=) on bytes memory element
//! - bytes_memory_compound_and: Compound AND assignment (&=) on bytes memory element
//! - bytes_memory_compound_xor: Compound XOR assignment (^=) on bytes memory element
//! - string_char_at_via_bytes_cast: Extracts a character from a string via bytes cast
//! - string_equal: Equality comparison (==) of two strings
//! - string_not_equal: Inequality comparison (!=) of two strings
//! - string_storage_indexed_byte_access: Accesses individual bytes of a storage string by index (0-9)
//! - bytes_storage_indexed_byte_access: Accesses individual bytes of storage bytes by index (0-9)
//! - bytes_memory_indexed_byte_access: Accesses individual bytes of memory bytes by index (0-9)

use crate::build_solidity;
use soroban_sdk::{Bytes, FromVal, IntoVal, String, Val};

fn bytes_eq(env: &soroban_sdk::Env, result: &Val, expected: &[u8]) -> bool {
    Bytes::from_val(env, result) == Bytes::from_slice(env, expected)
}

#[test]
fn returns_string_literal() {
    let src = build_solidity(
        r#"contract StringReturn {
            function greet() public pure returns (string memory) {
                return "hello";
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "greet", vec![]);
    let result_str = String::from_val(&src.env, &result);
    let expected_str = String::from_str(&src.env, "hello");
    assert_eq!(result_str, expected_str);
}

#[test]
fn string_length() {
    let src = build_solidity(
        r#"contract StringLength {
            function len(string memory s) public pure returns (uint64) {
                return uint64(bytes(s).length);
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let arg: Val = String::from_str(&src.env, "Solang!").into_val(&src.env);
    let result = src.invoke_contract(addr, "len", vec![arg]);
    let expected: Val = 7_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&result));

    let arg: Val = String::from_str(&src.env, "").into_val(&src.env);
    let result = src.invoke_contract(addr, "len", vec![arg]);
    let expected: Val = 0_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&result));
}

#[test]
fn string_storage_set_get() {
    let src = build_solidity(
        r#"contract StringStorage {
            string name;
            function setName(string memory n) public { name = n; }
            function getName() public view returns (string memory) { return name; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let input: Val = String::from_str(&src.env, "Solang").into_val(&src.env);
    src.invoke_contract(addr, "setName", vec![input]);

    let result = src.invoke_contract(addr, "getName", vec![]);
    let result_str = String::from_val(&src.env, &result);
    assert_eq!(result_str, String::from_str(&src.env, "Solang"));
}

#[test]
fn string_storage_overwrite() {
    let src = build_solidity(
        r#"contract StringStorage {
            string name;
            function setName(string memory n) public { name = n; }
            function getName() public view returns (string memory) { return name; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    src.invoke_contract(
        addr,
        "setName",
        vec![String::from_str(&src.env, "Alice").into_val(&src.env)],
    );
    src.invoke_contract(
        addr,
        "setName",
        vec![String::from_str(&src.env, "Bob").into_val(&src.env)],
    );

    let result = src.invoke_contract(addr, "getName", vec![]);
    assert_eq!(
        String::from_val(&src.env, &result),
        String::from_str(&src.env, "Bob"),
    );
}

#[test]
fn bytes_length() {
    let src = build_solidity(
        r#"contract BytesLen {
            function blen(bytes memory b) public pure returns (uint64) {
                return uint64(b.length);
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let arg: Val = Bytes::from_array(&src.env, &[0xAA, 0xBB, 0xCC]).into_val(&src.env);
    let result = src.invoke_contract(addr, "blen", vec![arg]);
    let expected: Val = 3_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&result));

    let arg: Val = Bytes::from_array(&src.env, &[]).into_val(&src.env);
    let result = src.invoke_contract(addr, "blen", vec![arg]);
    let expected: Val = 0_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&result));
}

#[test]
fn returns_string_from_var() {
    let src = build_solidity(
        r#"contract G {
            function greet() public pure returns (string memory) {
                string memory s = "very bad";
                s = "from var";
                return s;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "greet", vec![]);
    assert_eq!(
        String::from_val(&src.env, &result),
        String::from_str(&src.env, "from var"),
    );
}

#[test]
fn bytes_first_element() {
    let src = build_solidity(
        r#"contract B {
            function first(bytes memory b) public pure returns (uint64) {
                return uint64(uint8(b[0]));
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let arg: Val = Bytes::from_array(&src.env, &[0xAB, 0xCD]).into_val(&src.env);
    let result = src.invoke_contract(addr, "first", vec![arg]);
    let expected: Val = 0xAB_u64.into_val(&src.env);
    assert!(expected.shallow_eq(&result));
}

#[test]
fn bytes_storage_round_trip() {
    let src = build_solidity(
        r#"contract B {
            bytes data;
            function setData(bytes memory d) public { data = d; }
            function getData() public view returns (bytes memory) { return data; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let input: Val = Bytes::from_array(&src.env, &[1, 2, 3]).into_val(&src.env);
    src.invoke_contract(addr, "setData", vec![input]);

    let result = src.invoke_contract(addr, "getData", vec![]);
    assert!(bytes_eq(&src.env, &result, &[1, 2, 3]));
}

#[test]
fn string_storage_read_before_write() {
    let src = build_solidity(
        r#"contract N {
            string name;
            function getName() public view returns (string memory) { return name; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "getName", vec![]);
    assert_eq!(
        String::from_val(&src.env, &result),
        String::from_str(&src.env, ""),
    );
}

// ─── Memory bytes subscript write ────────────────────────────────────────────

#[test]
fn bytes_memory_subscript_write() {
    let src = build_solidity(
        r#"contract T {
            function set_byte() public pure returns (bytes memory) {
                bytes memory b = hex"aabbccddeeff";
                b[1] = 0x33;
                return b;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "set_byte", vec![]);
    assert!(bytes_eq(
        &src.env,
        &result,
        &[0xaa, 0x33, 0xcc, 0xdd, 0xee, 0xff]
    ));
}

// ─── Memory bytes compound assignment ────────────────────────────────────────

#[test]
fn bytes_memory_compound_or() {
    let src = build_solidity(
        r#"contract T {
            function op() public pure returns (bytes memory) {
                bytes memory b = hex"deadcafe";
                b[1] |= 0x50;   // 0xad | 0x50 = 0xfd
                return b;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "op", vec![]);
    assert!(bytes_eq(&src.env, &result, &[0xde, 0xfd, 0xca, 0xfe]));
}

#[test]
fn bytes_memory_compound_and() {
    let src = build_solidity(
        r#"contract T {
            function op() public pure returns (bytes memory) {
                bytes memory b = hex"deadcafe";
                b[3] &= 0x7f;   // 0xfe & 0x7f = 0x7e
                return b;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "op", vec![]);
    assert!(bytes_eq(&src.env, &result, &[0xde, 0xad, 0xca, 0x7e]));
}

#[test]
fn bytes_memory_compound_xor() {
    let src = build_solidity(
        r#"contract T {
            function op() public pure returns (bytes memory) {
                bytes memory b = hex"deadcafe";
                b[2] ^= 0xff;   // 0xca ^ 0xff = 0x35
                return b;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "op", vec![]);
    assert!(bytes_eq(&src.env, &result, &[0xde, 0xad, 0x35, 0xfe]));
}

#[test]
fn string_char_at_via_bytes_cast() {
    let src = build_solidity(
        r#"contract T {
            function char_at(string memory s) public pure returns (bytes memory) {
                bytes memory r = new bytes(1);
                r[0] = bytes(s)[2];
                return r;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let arg: Val = String::from_str(&src.env, "abcdef").into_val(&src.env);
    let result = src.invoke_contract(addr, "char_at", vec![arg]);
    assert!(bytes_eq(&src.env, &result, &[b'c']));
}

#[test]
fn string_equal() {
    let src = build_solidity(
        r#"contract StringCmp {
            function eq(string memory a, string memory b) public pure returns (bool) {
                return a == b;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let a: Val = String::from_str(&src.env, "hello").into_val(&src.env);
    let b: Val = String::from_str(&src.env, "hello").into_val(&src.env);
    let result = src.invoke_contract(addr, "eq", vec![a, b]);
    let expected: Val = true.into_val(&src.env);
    assert!(expected.shallow_eq(&result));

    let a: Val = String::from_str(&src.env, "hello").into_val(&src.env);
    let b: Val = String::from_str(&src.env, "world").into_val(&src.env);
    let result = src.invoke_contract(addr, "eq", vec![a, b]);
    let expected: Val = false.into_val(&src.env);
    assert!(expected.shallow_eq(&result));
}

#[test]
fn string_not_equal() {
    let src = build_solidity(
        r#"contract StringCmp {
            function neq(string memory a, string memory b) public pure returns (bool) {
                return a != b;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let a: Val = String::from_str(&src.env, "hello").into_val(&src.env);
    let b: Val = String::from_str(&src.env, "world").into_val(&src.env);
    let result = src.invoke_contract(addr, "neq", vec![a, b]);
    let expected: Val = true.into_val(&src.env);
    assert!(expected.shallow_eq(&result));

    let a: Val = String::from_str(&src.env, "hello").into_val(&src.env);
    let b: Val = String::from_str(&src.env, "hello").into_val(&src.env);
    let result = src.invoke_contract(addr, "neq", vec![a, b]);
    let expected: Val = false.into_val(&src.env);
    assert!(expected.shallow_eq(&result));
}

#[test]
fn string_storage_indexed_byte_access() {
    let src = build_solidity(
        r#"contract StringIndex {
            string digits = "0123456789";
            function byteAt(uint32 index) public view returns (uint32) {
                return uint32(bytes(digits)[index]);
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    for i in 0..10u32 {
        let arg: Val = i.into_val(&src.env);
        let result_val = src.invoke_contract(addr, "byteAt", vec![arg]);
        let expected_byte = (b'0' + i as u8) as u32;
        let result: u32 = FromVal::from_val(&src.env, &result_val);
        assert_eq!(expected_byte, result, "byte at index {i}");
    }
}

#[test]
fn bytes_storage_indexed_byte_access() {
    let src = build_solidity(
        r#"contract bytesIndex {
            bytes digits = "0123456789";
            function byteAt(uint32 index) public view returns (uint32) {
                return uint32(bytes(digits)[index]);
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    for i in 0..10u32 {
        let arg: Val = i.into_val(&src.env);
        let result_val = src.invoke_contract(addr, "byteAt", vec![arg]);
        let expected_byte = (b'0' + i as u8) as u32;
        let result: u32 = FromVal::from_val(&src.env, &result_val);
        assert_eq!(expected_byte, result, "byte at index {i}");
    }
}

#[test]
fn bytes_memory_indexed_byte_access() {
    let src = build_solidity(
        r#"contract bytesIndex {
            function byteAt(uint32 index) public view returns (uint32) {
                bytes digits = "0123456789";
                return uint32(digits[index]);
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    for i in 0..10u32 {
        let arg: Val = i.into_val(&src.env);
        let result_val = src.invoke_contract(addr, "byteAt", vec![arg]);
        let expected_byte = (b'0' + i as u8) as u32;
        let result: u32 = FromVal::from_val(&src.env, &result_val);
        assert_eq!(expected_byte, result, "byte at index {i}");
    }
}
