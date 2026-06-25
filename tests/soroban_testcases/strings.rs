// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{Bytes, FromVal, IntoVal, String, Val, U256};

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

#[test]
fn string_storage_length() {
    let src = build_solidity(
        r#"contract T {
            string name;
            function setName(string memory n) public { name = n; }
            function getLen() public view returns (uint256) { return name.length; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    src.invoke_contract(
        addr,
        "setName",
        vec![String::from_str(&src.env, "Solang").into_val(&src.env)],
    );
    let len = src.invoke_contract(addr, "getLen", vec![]);
    assert_eq!(
        U256::from_val(&src.env, &len),
        U256::from_u32(&src.env, 6),
        "getLen() must return 6 for \"Solang\""
    );

    src.invoke_contract(
        addr,
        "setName",
        vec![String::from_str(&src.env, "Hi").into_val(&src.env)],
    );
    let len2 = src.invoke_contract(addr, "getLen", vec![]);
    assert_eq!(
        U256::from_val(&src.env, &len2),
        U256::from_u32(&src.env, 2),
        "getLen() must reflect overwritten length"
    );
}

#[test]
fn bytes_storage_read_before_write() {
    let src = build_solidity(
        r#"contract T {
            bytes data;
            function getData() public view returns (bytes memory) { return data; }
            function getLen() public view returns (uint256) { return data.length; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    assert!(
        bytes_eq(&src.env, &src.invoke_contract(addr, "getData", vec![]), &[]),
        "getData() before any write must return empty bytes"
    );
    assert_eq!(
        U256::from_val(&src.env, &src.invoke_contract(addr, "getLen", vec![])),
        U256::from_u32(&src.env, 0),
        "getLen() before any write must return 0"
    );
}

#[test]
fn string_bytes_cast() {
    let src = build_solidity(
        r#"contract T {
            function to_string(bytes memory b) public pure returns (string memory) { return string(b); }
            function to_bytes(string memory s) public pure returns (bytes memory) { return bytes(s); }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let payload = [b'h', b'e', b'l', b'l', b'o'];
    let b: Val = Bytes::from_array(&src.env, &payload).into_val(&src.env);
    let result = src.invoke_contract(addr, "to_string", vec![b]);
    assert_eq!(
        String::from_val(&src.env, &result),
        String::from_str(&src.env, "hello"),
        "string(bytes) must produce the matching StringObject"
    );

    let s: Val = String::from_str(&src.env, "hello").into_val(&src.env);
    let result = src.invoke_contract(addr, "to_bytes", vec![s]);
    assert!(
        bytes_eq(&src.env, &result, &payload),
        "bytes(string) must produce raw UTF-8 bytes"
    );
}
