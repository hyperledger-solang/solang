// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, SorobanEnv};
use soroban_sdk::{Address, Bytes, BytesN, FromVal, IntoVal};

const SOURCE: &str = r#"contract BytesSubscript {
    bytes public data;

    function set_data(bytes memory d) public {
        data = d;
    }

    function get_byte(uint32 i) public view returns (bytes1) {
        return data[i];
    }

    function set_byte(uint32 i, bytes1 v) public {
        data[i] = v;
    }
}"#;

/// deploys the contract and sets `data` with `payload`. returns env and contract address.
fn setup(payload: &[u8]) -> (SorobanEnv, Address) {
    let src = build_solidity(SOURCE, |_| {});
    let addr = src.contracts.last().unwrap().clone();
    let payload_bytes = Bytes::from_slice(&src.env, payload);
    src.invoke_contract(&addr, "set_data", vec![payload_bytes.into_val(&src.env)]);
    (src, addr)
}

// solidity function get_byte wrapper
fn get_byte(src: &SorobanEnv, addr: &Address, i: u32) -> u8 {
    let raw = src.invoke_contract(addr, "get_byte", vec![i.into_val(&src.env)]);
    let byte = BytesN::<1>::from_val(&src.env, &raw);
    byte.to_array()[0]
}

// solidity function set_byte wrapper
fn set_byte(src: &SorobanEnv, addr: &Address, i: u32, v: u8) {
    let byte = BytesN::<1>::from_array(&src.env, &[v]);
    src.invoke_contract(
        addr,
        "set_byte",
        vec![i.into_val(&src.env), byte.into_val(&src.env)],
    );
}

#[test]
fn get_byte_reads_each_position() {
    let (src, addr) = setup(b"hello");

    assert_eq!(get_byte(&src, &addr, 0), b'h');
    assert_eq!(get_byte(&src, &addr, 1), b'e');
    assert_eq!(get_byte(&src, &addr, 2), b'l');
    assert_eq!(get_byte(&src, &addr, 3), b'l');
    assert_eq!(get_byte(&src, &addr, 4), b'o');
}

#[test]
fn set_byte_at_first_index() {
    let (src, addr) = setup(b"hello");

    set_byte(&src, &addr, 0, b'J');

    assert_eq!(get_byte(&src, &addr, 0), b'J');
}

#[test]
fn set_byte_at_last_index() {
    let (src, addr) = setup(b"hello");

    set_byte(&src, &addr, 4, b'!');

    assert_eq!(get_byte(&src, &addr, 4), b'!');
}

#[test]
fn set_byte_preserves_other_bytes() {
    let (src, addr) = setup(b"hello");

    set_byte(&src, &addr, 2, b'X');

    assert_eq!(get_byte(&src, &addr, 0), b'h');
    assert_eq!(get_byte(&src, &addr, 1), b'e');
    assert_eq!(get_byte(&src, &addr, 2), b'X');
    assert_eq!(get_byte(&src, &addr, 3), b'l');
    assert_eq!(get_byte(&src, &addr, 4), b'o');
}

#[test]
fn set_byte_repeated_keeps_latest() {
    let (src, addr) = setup(b"hello");

    set_byte(&src, &addr, 1, b'X');
    set_byte(&src, &addr, 1, b'Y');
    set_byte(&src, &addr, 1, b'Z');

    assert_eq!(get_byte(&src, &addr, 1), b'Z');
}

#[test]
fn get_byte_out_of_bounds_traps() {
    let (src, addr) = setup(b"hello");

    let logs = src.invoke_contract_expect_error(&addr, "get_byte", vec![5u32.into_val(&src.env)]);
    assert!(
        logs.iter()
            .any(|l| l.contains("storage bytes index out of bounds")),
        "expected OOB log, got: {logs:?}"
    );
}

#[test]
fn set_byte_out_of_bounds_traps() {
    let (src, addr) = setup(b"hello");

    let byte = BytesN::<1>::from_array(&src.env, &[b'X']);
    let logs = src.invoke_contract_expect_error(
        &addr,
        "set_byte",
        vec![5u32.into_val(&src.env), byte.into_val(&src.env)],
    );
    assert!(
        logs.iter()
            .any(|l| l.contains("storage bytes index out of bounds")),
        "expected OOB log, got: {logs:?}"
    );
}

#[test]
fn get_byte_on_empty_bytes_traps() {
    let (src, addr) = setup(b"");

    let logs = src.invoke_contract_expect_error(&addr, "get_byte", vec![0u32.into_val(&src.env)]);
    assert!(
        logs.iter()
            .any(|l| l.contains("storage bytes index out of bounds")),
        "expected OOB log, got: {logs:?}"
    );
}
