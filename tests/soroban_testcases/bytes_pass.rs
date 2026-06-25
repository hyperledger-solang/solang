// SPDX-License-Identifier: Apache-2.0
use crate::build_solidity;
use soroban_sdk::testutils::Events as _;
use soroban_sdk::{Bytes, BytesN, FromVal, IntoVal, U256, Val};

fn bytes_eq(env: &soroban_sdk::Env, result: &Val, expected: &[u8]) -> bool {
    Bytes::from_val(env, result) == Bytes::from_slice(env, expected)
}

#[test]
fn bytesn_index_read_literal() {
    let src = build_solidity(
        r#"contract T {
            function f() public pure returns (bool) {
                bytes4 x = 0xDEADBEEF;
                return x[0] == 0xDE && x[1] == 0xAD && x[3] == 0xEF;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "f", vec![]);
    let expected: Val = true.into_val(&src.env);
    assert!(expected.shallow_eq(&result));
}

#[test]
fn bytesn_length_is_constant() {
    let src = build_solidity(
        r#"contract T {
            function f() public pure returns (uint32) { bytes32 x; return x.length; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "f", vec![]);
    let expected: Val = 32_u32.into_val(&src.env);
    assert!(expected.shallow_eq(&result));
}

#[test]
fn bytesn_bitwise_ops() {
    let src = build_solidity(
        r#"contract T {
            function f() public pure returns (bool) {
                bytes4 a = 0xF0F0F0F0; bytes4 b = 0x0FF00FF0;
                return (a & b) == 0x00F000F0 && (a | b) == 0xFFF0FFF0
                    && (a ^ b) == 0xFF00FF00 && (~a)    == 0x0F0F0F0F;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "f", vec![]);
    let expected: Val = true.into_val(&src.env);
    assert!(expected.shallow_eq(&result));
}

#[test]
fn bytesn_shift_ops() {
    let src = build_solidity(
        r#"contract T {
            function f() public pure returns (bool) {
                bytes4 a = 0x12345678;
                return (a << 8) == 0x34567800 && (a >> 8) == 0x00123456;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "f", vec![]);
    let expected: Val = true.into_val(&src.env);
    assert!(expected.shallow_eq(&result));
}

#[test]
fn bytesn_compare_and_whole_value_assign() {
    let src = build_solidity(
        r#"contract T {
            function f() public pure returns (bool) {
                bytes4 a = 0x11223344; bytes4 b = a; bytes4 c = 0x11223345;
                return a == b && a != c && a < c;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "f", vec![]);
    let expected: Val = true.into_val(&src.env);
    assert!(expected.shallow_eq(&result));
}

#[test]
fn bytesn_casts() {
    let src = build_solidity(
        r#"contract T {
            function f() public pure returns (bool) {
                bytes32 b = bytes32(uint256(0xABCD));
                return uint256(b) == 0xABCD && bytes4(b) == 0x00000000;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "f", vec![]);
    let expected: Val = true.into_val(&src.env);
    assert!(expected.shallow_eq(&result));
}

#[test]
fn bytes1_abi_round_trip() {
    let src = build_solidity(
        r#"contract T {
            function echo(bytes1 b) public pure returns (bytes1) { return b; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let arg: Val = BytesN::from_array(&src.env, &[0xAB]).into_val(&src.env);
    let result = src.invoke_contract(addr, "echo", vec![arg]);
    assert!(bytes_eq(&src.env, &result, &[0xAB]));
}

#[test]
fn bytes4_abi_round_trip() {
    let src = build_solidity(
        r#"contract T {
            function echo(bytes4 b) public pure returns (bytes4) { return b; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let arg: Val = BytesN::from_array(&src.env, &[0xAA, 0xBB, 0xCC, 0xDD]).into_val(&src.env);
    let result = src.invoke_contract(addr, "echo", vec![arg]);
    assert!(bytes_eq(&src.env, &result, &[0xAA, 0xBB, 0xCC, 0xDD]));
}

#[test]
fn bytes32_abi_round_trip() {
    let src = build_solidity(
        r#"contract T {
            function echo(bytes32 b) public pure returns (bytes32) { return b; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let payload: [u8; 32] = core::array::from_fn(|i| i as u8);
    let arg: Val = BytesN::from_array(&src.env, &payload).into_val(&src.env);
    let result = src.invoke_contract(addr, "echo", vec![arg]);
    assert!(bytes_eq(&src.env, &result, &payload));
}

#[test]
fn bytesn_byte_order_index_zero_is_first_wire_byte() {
    // bytes4 x = 0xAABBCCDD → x[0] == 0xAA (most-significant byte).
    // On the wire the BytesObject must be [0xAA, 0xBB, 0xCC, 0xDD].
    let src = build_solidity(
        r#"contract T {
            function first_byte(bytes4 b) public pure returns (bytes1) {
                return b[0];
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let arg: Val = BytesN::from_array(&src.env, &[0xAA, 0xBB, 0xCC, 0xDD]).into_val(&src.env);
    let result = src.invoke_contract(addr, "first_byte", vec![arg]);
    assert!(bytes_eq(&src.env, &result, &[0xAA]), "b[0] must be the first wire byte (0xAA)");
}

#[test]
fn bytesn_abi_edge_values() {
    // all-zero and all-0xFF must round-trip cleanly
    let src = build_solidity(
        r#"contract T {
            function echo(bytes4 b) public pure returns (bytes4) { return b; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let zeros: Val = BytesN::from_array(&src.env, &[0x00; 4]).into_val(&src.env);
    let result = src.invoke_contract(addr, "echo", vec![zeros]);
    assert!(bytes_eq(&src.env, &result, &[0x00; 4]));

    let ones: Val = BytesN::from_array(&src.env, &[0xFF; 4]).into_val(&src.env);
    let result = src.invoke_contract(addr, "echo", vec![ones]);
    assert!(bytes_eq(&src.env, &result, &[0xFF; 4]));
}

#[test]
fn bytesn_literal_init_returned() {
    // bytes4 x = 0xAABBCCDD as a constant literal returned directly.
    let src = build_solidity(
        r#"contract T {
            function get() public pure returns (bytes4) {
                bytes4 x = 0xAABBCCDD;
                return x;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let result = src.invoke_contract(addr, "get", vec![]);
    assert!(bytes_eq(&src.env, &result, &[0xAA, 0xBB, 0xCC, 0xDD]));
}

#[test]
fn bytes_abi_echo_with_embedded_zeros() {
    let src = build_solidity(
        r#"contract T {
            function echo(bytes memory x) public pure returns (bytes memory) { return x; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let payload = [0x00u8, 0x01, 0xFF, 0x00, 0x7F];
    let v: Val = soroban_sdk::Bytes::from_array(&src.env, &payload).into_val(&src.env);
    let result = src.invoke_contract(addr, "echo", vec![v]);
    assert!(bytes_eq(&src.env, &result, &payload));
}

#[test]
fn bytes_abi_empty_echo() {
    let src = build_solidity(
        r#"contract T {
            function echo(bytes memory x) public pure returns (bytes memory) { return x; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let v: Val = soroban_sdk::Bytes::from_array(&src.env, &[]).into_val(&src.env);
    let result = src.invoke_contract(addr, "echo", vec![v]);
    assert!(bytes_eq(&src.env, &result, &[]));
}

#[test]
fn bytes_abi_mutate_then_return() {
    // host→guest decode, mutate in place via XOR (avoids uint8/uint32 sema issue), re-encode.
    let src = build_solidity(
        r#"contract T {
            function flip(bytes memory x) public pure returns (bytes memory) {
                x[0] = x[0] ^ 0xFF;
                return x;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let v: Val = soroban_sdk::Bytes::from_array(&src.env, &[0xAA, 0x00, 0xFF]).into_val(&src.env);
    let result = src.invoke_contract(addr, "flip", vec![v]);
    assert!(bytes_eq(&src.env, &result, &[0x55, 0x00, 0xFF]));
}

#[test]
fn bytesn_reinterpret_as_uint_across_abi() {
    // host BytesN<4> → guest bytes4 → uint32: verifies the decode reversal end-to-end.
    let src = build_solidity(
        r#"contract T {
            function asUint(bytes4 x) public pure returns (uint32) { return uint32(x); }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let v: Val = BytesN::from_array(&src.env, &[0x12, 0x34, 0x56, 0x78]).into_val(&src.env);
    let result = src.invoke_contract(addr, "asUint", vec![v]);
    let expected: Val = 0x12345678_u32.into_val(&src.env);
    assert!(expected.shallow_eq(&result));
}

#[test]
fn mixed_bytesn_and_dynamic_params() {
    // One call carrying both a static bytes4 and a dynamic bytes argument.
    // Uses bytes4→uint32 cast (not uint8) to avoid the Soroban uint8→uint32 sema rejection.
    let src = build_solidity(
        r#"contract T {
            function combine(bytes4 tag, bytes memory data) public pure returns (uint32) {
                return uint32(tag) + uint32(data.length);
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let tag:  Val = BytesN::from_array(&src.env, &[0x00, 0x00, 0x00, 0x09]).into_val(&src.env);
    let data: Val = soroban_sdk::Bytes::from_array(&src.env, &[0, 0, 0]).into_val(&src.env);
    let result = src.invoke_contract(addr, "combine", vec![tag, data]);
    let expected: Val = 12_u32.into_val(&src.env);
    assert!(expected.shallow_eq(&result));
}

#[test]
fn bytes32_storage_round_trip_and_overwrite() {
    let src = build_solidity(
        r#"contract T {
            bytes32 h;
            function setH(bytes32 x) public { h = x; }
            function getH() public view returns (bytes32) { return h; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let a: [u8; 32] = {
        let mut arr = [0u8; 32];
        arr[0] = 0xAA;
        arr[31] = 0x01;
        arr
    };
    src.invoke_contract(addr, "setH", vec![BytesN::from_array(&src.env, &a).into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &src.invoke_contract(addr, "getH", vec![]), &a));
    let b = [0x42u8; 32];
    src.invoke_contract(addr, "setH", vec![BytesN::from_array(&src.env, &b).into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &src.invoke_contract(addr, "getH", vec![]), &b));
}

#[test]
fn bytes4_storage_read_before_write_is_zero() {
    let src = build_solidity(
        r#"contract T {
            bytes4 m;
            function getM() public view returns (bytes4) { return m; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    assert!(bytes_eq(&src.env, &src.invoke_contract(addr, "getM", vec![]), &[0, 0, 0, 0]));
}

#[test]
fn bytes4_state_var_literal_initializer() {
    let src = build_solidity(
        r#"contract T {
            bytes4 m = 0x12345678;
            function getM() public view returns (bytes4) { return m; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    assert!(bytes_eq(
        &src.env,
        &src.invoke_contract(addr, "getM", vec![]),
        &[0x12, 0x34, 0x56, 0x78]
    ));
}

#[test]
fn bytes32_mapping_round_trip_written_key() {
    // Gap D: unwritten keys trap; only test a written key.
    let src = build_solidity(
        r#"contract T {
            mapping(uint64 => bytes32) m;
            function setM(uint64 k, bytes32 v) public { m[k] = v; }
            function getM(uint64 k) public view returns (bytes32) { return m[k]; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let a = [0xABu8; 32];
    let k: Val = 7_u64.into_val(&src.env);
    src.invoke_contract(addr, "setM", vec![k, BytesN::from_array(&src.env, &a).into_val(&src.env)]);
    assert!(bytes_eq(
        &src.env,
        &src.invoke_contract(addr, "getM", vec![7_u64.into_val(&src.env)]),
        &a
    ));
}

#[test]
fn bytes_storage_subscript_write() {
    // Initialize storage bytes, write one byte via direct subscript, verify with whole-array read.
    let src = build_solidity(
        r#"contract T {
            bytes data;
            function setData(bytes memory d) public { data = d; }
            function getData() public view returns (bytes memory) { return data; }
            function writeAt() public { data[0] = 0x42; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    src.invoke_contract(
        addr,
        "setData",
        vec![soroban_sdk::Bytes::from_array(&src.env, &[0xAA, 0xBB, 0xCC]).into_val(&src.env)],
    );
    src.invoke_contract(addr, "writeAt", vec![]);
    let result = src.invoke_contract(addr, "getData", vec![]);
    assert!(bytes_eq(&src.env, &result, &[0x42, 0xBB, 0xCC]));
}

#[test]
fn bytes_storage_subscript_read() {
    // setData → readAt each index → writeAt each index → readAt each index again.
    let src = build_solidity(
        r#"contract T {
            bytes data;
            function setData(bytes memory d) public { data = d; }
            function readAt(uint32 i) public view returns (bytes1) { return data[i]; }
            function writeAt(uint32 i, bytes1 v) public { data[i] = v; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let initial: [u8; 5] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
    let overwrite: [u8; 5] = [0x11, 0x22, 0x33, 0x44, 0x55];

    src.invoke_contract(
        addr,
        "setData",
        vec![soroban_sdk::Bytes::from_array(&src.env, &initial).into_val(&src.env)],
    );

    for (i, &byte) in initial.iter().enumerate() {
        let result = src.invoke_contract(addr, "readAt", vec![(i as u32).into_val(&src.env)]);
        assert!(bytes_eq(&src.env, &result, &[byte]), "initial read at {i}: expected {byte:#04x}");
    }

    for (i, &byte) in overwrite.iter().enumerate() {
        src.invoke_contract(
            addr,
            "writeAt",
            vec![
                (i as u32).into_val(&src.env),
                BytesN::from_array(&src.env, &[byte]).into_val(&src.env),
            ],
        );
    }

    for (i, &byte) in overwrite.iter().enumerate() {
        let result = src.invoke_contract(addr, "readAt", vec![(i as u32).into_val(&src.env)]);
        assert!(bytes_eq(&src.env, &result, &[byte]), "post-write read at {i}: expected {byte:#04x}");
    }
}

#[test]
fn bytes32_public_accessor() {
    let src = build_solidity(
        r#"contract T {
            bytes32 public h;
            function setH(bytes32 x) public { h = x; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    let a = [0x5Au8; 32];
    src.invoke_contract(addr, "setH", vec![BytesN::from_array(&src.env, &a).into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &src.invoke_contract(addr, "h", vec![]), &a));
}

#[test]
fn bytes_public_accessor() {
    let src = build_solidity(
        r#"contract T {
            bytes public data;
            function setData(bytes memory d) public { data = d; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    src.invoke_contract(
        addr,
        "setData",
        vec![soroban_sdk::Bytes::from_array(&src.env, &[1, 2, 3]).into_val(&src.env)],
    );
    assert!(bytes_eq(&src.env, &src.invoke_contract(addr, "data", vec![]), &[1, 2, 3]));
}

#[test]
fn bytes32_event() {
    let src = build_solidity(
        r#"contract T {
            event H(bytes32 value);
            function go() public { emit H(bytes32(uint256(0xAB))); }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    src.invoke_contract(addr, "go", vec![]);
    let events = src.env.events().all();
    assert_eq!(events.len(), 1);
    let (_, topics, data) = events.get(0).unwrap();
    assert_eq!(topics.len(), 0);
    let mut expected = [0u8; 32];
    expected[31] = 0xAB;
    assert!(bytes_eq(&src.env, &data, &expected));
}

#[test]
fn bytes_event() {
    let src = build_solidity(
        r#"contract T {
            event B(bytes value);
            function go() public { emit B(hex"010203"); }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();
    src.invoke_contract(addr, "go", vec![]);
    let (_, _topics, data) = src.env.events().all().get(0).unwrap();
    assert!(bytes_eq(&src.env, &data, &[1, 2, 3]));
}

#[test]
fn bytes_storage_set_get_length() {
    let src = build_solidity(
        r#"contract T {
            bytes h;
            function setH(bytes calldata x) public { h = x; }
            function getH() public view returns (bytes memory) { return h; }
            function getL() public view returns (uint256) { return h.length; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let payload: [u8; 5] = [0x11, 0x22, 0x33, 0x44, 0x55];
    src.invoke_contract(
        addr,
        "setH",
        vec![soroban_sdk::Bytes::from_array(&src.env, &payload).into_val(&src.env)],
    );

    assert!(
        bytes_eq(&src.env, &src.invoke_contract(addr, "getH", vec![]), &payload),
        "getH must return the stored bytes"
    );

    let len_val = src.invoke_contract(addr, "getL", vec![]);
    assert_eq!(
        U256::from_val(&src.env, &len_val),
        U256::from_u32(&src.env, 5),
        "getL() must return the byte length as uint256"
    );

    let payload2: [u8; 3] = [0xAA, 0xBB, 0xCC];
    src.invoke_contract(
        addr,
        "setH",
        vec![soroban_sdk::Bytes::from_array(&src.env, &payload2).into_val(&src.env)],
    );
    assert!(
        bytes_eq(&src.env, &src.invoke_contract(addr, "getH", vec![]), &payload2),
        "getH must reflect overwritten bytes"
    );
    let len_val2 = src.invoke_contract(addr, "getL", vec![]);
    assert_eq!(
        U256::from_val(&src.env, &len_val2),
        U256::from_u32(&src.env, 3),
        "getL() must reflect overwritten length"
    );
}

#[test]
fn bytes_storage_compound_ops() {
    let src = build_solidity(
        r#"contract T {
            bytes data;
            function setData(bytes memory d) public { data = d; }
            function getData() public view returns (bytes memory) { return data; }
            function orAt(uint32 i, bytes1 mask)  public { data[i] |= mask; }
            function andAt(uint32 i, bytes1 mask) public { data[i] &= mask; }
            function xorAt(uint32 i, bytes1 mask) public { data[i] ^= mask; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    src.invoke_contract(
        addr,
        "setData",
        vec![soroban_sdk::Bytes::from_array(&src.env, &[0xF0, 0xFF, 0xAA]).into_val(&src.env)],
    );

    src.invoke_contract(addr, "orAt",  vec![0_u32.into_val(&src.env), BytesN::from_array(&src.env, &[0x0F]).into_val(&src.env)]);
    src.invoke_contract(addr, "andAt", vec![1_u32.into_val(&src.env), BytesN::from_array(&src.env, &[0x0F]).into_val(&src.env)]);
    src.invoke_contract(addr, "xorAt", vec![2_u32.into_val(&src.env), BytesN::from_array(&src.env, &[0xFF]).into_val(&src.env)]);

    assert!(
        bytes_eq(&src.env, &src.invoke_contract(addr, "getData", vec![]), &[0xFF, 0x0F, 0x55]),
        "compound ops |= &= ^= must update storage bytes in place"
    );
}

#[test]
fn bytes_equal() {
    let src = build_solidity(
        r#"contract T {
            function eq(bytes memory a, bytes memory b) public pure returns (bool) { return a == b; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let same_a: Val = soroban_sdk::Bytes::from_array(&src.env, &[0x01, 0x02, 0x03]).into_val(&src.env);
    let same_b: Val = soroban_sdk::Bytes::from_array(&src.env, &[0x01, 0x02, 0x03]).into_val(&src.env);
    let result = src.invoke_contract(addr, "eq", vec![same_a, same_b]);
    let t: Val = true.into_val(&src.env);
    assert!(t.shallow_eq(&result), "equal bytes must return true");

    let diff_a: Val = soroban_sdk::Bytes::from_array(&src.env, &[0x01, 0x02, 0x03]).into_val(&src.env);
    let diff_b: Val = soroban_sdk::Bytes::from_array(&src.env, &[0x01, 0x02, 0xFF]).into_val(&src.env);
    let result = src.invoke_contract(addr, "eq", vec![diff_a, diff_b]);
    let f: Val = false.into_val(&src.env);
    assert!(f.shallow_eq(&result), "different bytes must return false");

    let empty_a: Val = soroban_sdk::Bytes::from_array(&src.env, &[]).into_val(&src.env);
    let empty_b: Val = soroban_sdk::Bytes::from_array(&src.env, &[]).into_val(&src.env);
    let result = src.invoke_contract(addr, "eq", vec![empty_a, empty_b]);
    let t: Val = true.into_val(&src.env);
    assert!(t.shallow_eq(&result), "two empty bytes must be equal");
}

#[test]
fn bytes_not_equal() {
    let src = build_solidity(
        r#"contract T {
            function neq(bytes memory a, bytes memory b) public pure returns (bool) { return a != b; }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let diff_a: Val = soroban_sdk::Bytes::from_array(&src.env, &[0xAA, 0xBB]).into_val(&src.env);
    let diff_b: Val = soroban_sdk::Bytes::from_array(&src.env, &[0xAA, 0xCC]).into_val(&src.env);
    let result = src.invoke_contract(addr, "neq", vec![diff_a, diff_b]);
    let t: Val = true.into_val(&src.env);
    assert!(t.shallow_eq(&result), "different bytes must return true for !=");

    let same_a: Val = soroban_sdk::Bytes::from_array(&src.env, &[0xAA, 0xBB]).into_val(&src.env);
    let same_b: Val = soroban_sdk::Bytes::from_array(&src.env, &[0xAA, 0xBB]).into_val(&src.env);
    let result = src.invoke_contract(addr, "neq", vec![same_a, same_b]);
    let f: Val = false.into_val(&src.env);
    assert!(f.shallow_eq(&result), "equal bytes must return false for !=");

    let short: Val = soroban_sdk::Bytes::from_array(&src.env, &[0x01]).into_val(&src.env);
    let long:  Val = soroban_sdk::Bytes::from_array(&src.env, &[0x01, 0x02]).into_val(&src.env);
    let result = src.invoke_contract(addr, "neq", vec![short, long]);
    let t: Val = true.into_val(&src.env);
    assert!(t.shallow_eq(&result), "different-length bytes must return true for !=");
}

#[test]
fn bytesn_to_dynamic_cast() {
    let src = build_solidity(
        r#"contract T {
            function to_dyn(bytes4 x) public pure returns (bytes memory) { return bytes(x); }
            function to_n(bytes memory x) public pure returns (bytes4) { return bytes4(x); }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let arg: Val = BytesN::from_array(&src.env, &[0xDE, 0xAD, 0xBE, 0xEF]).into_val(&src.env);
    let result = src.invoke_contract(addr, "to_dyn", vec![arg]);
    assert!(bytes_eq(&src.env, &result, &[0xDE, 0xAD, 0xBE, 0xEF]), "bytes(bytes4) must preserve big-endian byte order");

    let payload: Val = soroban_sdk::Bytes::from_array(&src.env, &[0x11, 0x22, 0x33, 0x44]).into_val(&src.env);
    let result = src.invoke_contract(addr, "to_n", vec![payload]);
    assert!(bytes_eq(&src.env, &result, &[0x11, 0x22, 0x33, 0x44]), "bytes4(bytes memory) must round-trip via beNtoleN");
}

#[test]
fn new_bytes_runtime_size() {
    let src = build_solidity(
        r#"contract T {
            function alloc(uint32 n) public pure returns (bytes memory) {
                bytes memory b = new bytes(n);
                for (uint32 i = 0; i < n; i++) {
                    b[i] = 0x42;
                }
                return b;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let result = src.invoke_contract(addr, "alloc", vec![0_u32.into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &result, &[]), "alloc(0) must return empty bytes");

    let result = src.invoke_contract(addr, "alloc", vec![3_u32.into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &result, &[0x42, 0x42, 0x42]), "alloc(3) must fill 0x42");

    let result = src.invoke_contract(addr, "alloc", vec![5_u32.into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &result, &[0x42; 5]), "alloc(5) must fill 0x42");
}

#[test]
fn bytes_memory_push() {
    let src = build_solidity(
        r#"contract T {
            function push_one(bytes memory b, bytes1 v) public pure returns (bytes memory) {
                b.push(v);
                return b;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let b: Val = soroban_sdk::Bytes::from_array(&src.env, &[0x01, 0x02]).into_val(&src.env);
    let v: Val = BytesN::from_array(&src.env, &[0x03]).into_val(&src.env);
    let result = src.invoke_contract(addr, "push_one", vec![b, v]);
    assert!(bytes_eq(&src.env, &result, &[0x01, 0x02, 0x03]), "push 0x03 onto [0x01,0x02] must give [0x01,0x02,0x03]");

    let empty: Val = soroban_sdk::Bytes::from_array(&src.env, &[]).into_val(&src.env);
    let v2: Val = BytesN::from_array(&src.env, &[0xAA]).into_val(&src.env);
    let result = src.invoke_contract(addr, "push_one", vec![empty, v2]);
    assert!(bytes_eq(&src.env, &result, &[0xAA]), "push 0xAA onto [] must give [0xAA]");
}

#[test]
fn bytes_memory_push_loop() {
    let src = build_solidity(
        r#"contract T {
            function build(bytes1 v, uint32 n) public pure returns (bytes memory) {
                bytes memory b = new bytes(0);
                for (uint32 i = 0; i < n; i++) {
                    b.push(v);
                }
                return b;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let v: Val = BytesN::from_array(&src.env, &[0x42]).into_val(&src.env);
    let result = src.invoke_contract(addr, "build", vec![v, 0_u32.into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &result, &[]), "push loop n=0 must give empty bytes");

    let v: Val = BytesN::from_array(&src.env, &[0x42]).into_val(&src.env);
    let result = src.invoke_contract(addr, "build", vec![v, 4_u32.into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &result, &[0x42; 4]), "push loop n=4 must give [0x42;4]");

    let v: Val = BytesN::from_array(&src.env, &[0xBE]).into_val(&src.env);
    let result = src.invoke_contract(addr, "build", vec![v, 6_u32.into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &result, &[0xBE; 6]), "push loop n=6 must give [0xBE;6]");
}

#[test]
fn bytes_memory_pop() {
    let src = build_solidity(
        r#"contract T {
            function pop_one(bytes memory b) public pure returns (bytes memory) {
                b.pop();
                return b;
            }
            function pop_two(bytes memory b) public pure returns (bytes memory) {
                b.pop();
                b.pop();
                return b;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let b: Val = soroban_sdk::Bytes::from_array(&src.env, &[0xAA, 0xBB, 0xCC]).into_val(&src.env);
    let result = src.invoke_contract(addr, "pop_one", vec![b]);
    assert!(bytes_eq(&src.env, &result, &[0xAA, 0xBB]), "pop_one must remove last byte");

    let b: Val = soroban_sdk::Bytes::from_array(&src.env, &[0x11, 0x22, 0x33, 0x44]).into_val(&src.env);
    let result = src.invoke_contract(addr, "pop_two", vec![b]);
    assert!(bytes_eq(&src.env, &result, &[0x11, 0x22]), "pop_two must remove last two bytes");

    let b: Val = soroban_sdk::Bytes::from_array(&src.env, &[0xFF]).into_val(&src.env);
    let result = src.invoke_contract(addr, "pop_one", vec![b]);
    assert!(bytes_eq(&src.env, &result, &[]), "pop_one on single-byte must give empty");
}

#[test]
fn bytes_memory_pop_loop() {
    let src = build_solidity(
        r#"contract T {
            function trim(bytes memory b, uint32 n) public pure returns (bytes memory) {
                for (uint32 i = 0; i < n; i++) {
                    b.pop();
                }
                return b;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let initial = [0x10u8, 0x20, 0x30, 0x40, 0x50];

    let b: Val = soroban_sdk::Bytes::from_array(&src.env, &initial).into_val(&src.env);
    let result = src.invoke_contract(addr, "trim", vec![b, 0_u32.into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &result, &initial), "trim 0 must leave bytes unchanged");

    let b: Val = soroban_sdk::Bytes::from_array(&src.env, &initial).into_val(&src.env);
    let result = src.invoke_contract(addr, "trim", vec![b, 1_u32.into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &result, &initial[..4]), "trim 1 must remove last byte");

    let b: Val = soroban_sdk::Bytes::from_array(&src.env, &initial).into_val(&src.env);
    let result = src.invoke_contract(addr, "trim", vec![b, 3_u32.into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &result, &initial[..2]), "trim 3 must leave first two bytes");

    let b: Val = soroban_sdk::Bytes::from_array(&src.env, &initial).into_val(&src.env);
    let result = src.invoke_contract(addr, "trim", vec![b, 5_u32.into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &result, &[]), "trim all must give empty bytes");
}

#[test]
fn bytes_memory_push_pop_roundtrip() {
    let src = build_solidity(
        r#"contract T {
            function roundtrip(bytes memory b, bytes1 v, uint32 n) public pure returns (bytes memory) {
                for (uint32 i = 0; i < n; i++) {
                    b.push(v);
                }
                for (uint32 i = 0; i < n; i++) {
                    b.pop();
                }
                return b;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let original = [0xCAu8, 0xFE, 0xBA, 0xBE];

    let b: Val = soroban_sdk::Bytes::from_array(&src.env, &original).into_val(&src.env);
    let v: Val = BytesN::from_array(&src.env, &[0x00]).into_val(&src.env);
    let result = src.invoke_contract(addr, "roundtrip", vec![b, v, 0_u32.into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &result, &original), "roundtrip n=0 must be identity");

    let b: Val = soroban_sdk::Bytes::from_array(&src.env, &original).into_val(&src.env);
    let v: Val = BytesN::from_array(&src.env, &[0xFF]).into_val(&src.env);
    let result = src.invoke_contract(addr, "roundtrip", vec![b, v, 3_u32.into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &result, &original), "roundtrip n=3 must restore original");

    let empty: Val = soroban_sdk::Bytes::from_array(&src.env, &[]).into_val(&src.env);
    let v: Val = BytesN::from_array(&src.env, &[0xAB]).into_val(&src.env);
    let result = src.invoke_contract(addr, "roundtrip", vec![empty, v, 5_u32.into_val(&src.env)]);
    assert!(bytes_eq(&src.env, &result, &[]), "roundtrip on empty n=5 must give empty");
}
