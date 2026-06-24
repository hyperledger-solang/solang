// SPDX-License-Identifier: Apache-2.0

//! Passing runtime tests for Soroban `bytes` (dynamic) and `bytesN` (fixed-size).
//!
//! Test coverage:
//!
//! **bytesN body ops** (no ABI crossing — guards Solidity semantics of fixed bytes in the body)
//! - bytesn_index_read_literal: `bytes4 x = 0xDEADBEEF; x[0] == 0xDE`, MSB is index 0
//! - bytesn_length_is_constant: `bytes32 x; x.length == 32`
//! - bytesn_bitwise_ops: `& | ^ ~` on `bytes4` literals
//! - bytesn_shift_ops: `<< 8` and `>> 8` on `bytes4`
//! - bytesn_compare_and_whole_value_assign: `==`, `!=`, `<` and copy-assign `b = a`
//! - bytesn_casts: `bytes32(uint256(v))`, `uint256(b)`, truncating `bytes4(b32)`
//!
//! **bytesN ABI round-trips** (Phase 1 — host BytesN↔guest iN via `BytesCast` + `b.3`/`b.1`)
//! - bytes1_abi_round_trip: `bytes1` echo: `[0xAB]` → guest → `[0xAB]`
//! - bytes4_abi_round_trip: `bytes4` echo: `[0xDE,0xAD,0xBE,0xEF]` → guest → same
//! - bytes32_abi_round_trip: `bytes32` echo: `[0x00..0x1F]` → guest → same
//! - bytesn_byte_order_index_zero_is_first_wire_byte: wire byte 0 == Solidity `x[0]` (MSB)
//! - bytesn_abi_edge_values: all-`0x00` and all-`0xFF` `bytes32` round-trip
//! - bytesn_literal_init_returned: `bytes4 x = 0xAABBCCDD; return x;` encodes `[AA,BB,CC,DD]`
//!
//! **dynamic bytes host↔guest round-trips** (Phase 1 — `bytes memory` ABI codec end-to-end)
//! - bytes_abi_echo_with_embedded_zeros: interior `0x00` bytes survive encode/decode
//! - bytes_abi_empty_echo: empty `bytes memory` round-trips as zero-length `BytesObject`
//! - bytes_abi_mutate_then_return: host→guest decode, XOR mutation, re-encode observed by host
//! - bytesn_reinterpret_as_uint_across_abi: host `BytesN<4>` → `uint32(bytes4)` value check
//! - mixed_bytesn_and_dynamic_params: one call with both `bytes4` and `bytes memory` params
//!
//! **bytesN storage** (Phase 2 — `default_storage_value` + `type_to_tagged_zero_val`)
//! - bytes32_storage_round_trip_and_overwrite: set → get → overwrite → get on `bytes32`
//! - bytes4_storage_read_before_write_is_zero: unwritten `bytes4` state var reads as `0x00000000`
//! - bytes4_state_var_literal_initializer: `bytes4 m = 0x12345678;` persists across calls
//! - bytes32_mapping_round_trip_written_key: `mapping(uint64 => bytes32)` set → get on written key
//!
//! **accessor & event consistency** (Phase 3 — Gap B gates removed)
//! - bytes32_public_accessor: `bytes32 public h;` auto-getter returns correct value
//! - bytes_public_accessor: `bytes public data;` auto-getter returns correct value
//! - bytes32_event: `event H(bytes32)` emits with correct byte order
//! - bytes_event: `event B(bytes)` emits correct dynamic bytes payload

use crate::build_solidity;
use soroban_sdk::testutils::Events as _;
use soroban_sdk::{Bytes, BytesN, FromVal, IntoVal, Val};

fn bytes_eq(env: &soroban_sdk::Env, result: &Val, expected: &[u8]) -> bool {
    Bytes::from_val(env, result) == Bytes::from_slice(env, expected)
}

// ─── Phase 1: bytesN body ops (no ABI bytesN crossing) ──────────────────────

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

// ─── Phase 1: bytesN ABI round-trips ─────────────────────────────────────────

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

// ─── Phase 1: dynamic bytes host↔guest round-trips ────────────────────────────

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

// ─── Phase 2: bytesN storage ──────────────────────────────────────────────────

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

// ─── Phase 3: accessor & event consistency (Gap B) ────────────────────────────

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
    // bytes32(uint256(0xAB)) → byte 0 is MSB = 0x00, byte 31 = 0xAB
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
