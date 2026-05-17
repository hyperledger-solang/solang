// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{BytesN, FromVal, IntoVal};

#[test]
fn echo_bytes1() {
    let src = build_solidity(
        r#"contract BytesNCodec {
            function echo_bytes1(bytes1 x) public pure returns (bytes1) {
                return x;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let payload = BytesN::<1>::from_array(&src.env, &[0xAB]);
    let res = src.invoke_contract(
        addr,
        "echo_bytes1",
        vec![payload.clone().into_val(&src.env)],
    );
    assert_eq!(BytesN::<1>::from_val(&src.env, &res), payload);
}

#[test]
fn echo_bytes5() {
    let src = build_solidity(
        r#"contract BytesNCodec {
            function echo_bytes5(bytes5 x) public pure returns (bytes5) {
                return x;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let payload = BytesN::<5>::from_array(&src.env, &[0x01, 0x02, 0x03, 0x04, 0x05]);
    let res = src.invoke_contract(
        addr,
        "echo_bytes5",
        vec![payload.clone().into_val(&src.env)],
    );
    assert_eq!(BytesN::<5>::from_val(&src.env, &res), payload);
}

#[test]
fn echo_bytes32() {
    let src = build_solidity(
        r#"contract BytesNCodec {
            function echo_bytes32(bytes32 x) public pure returns (bytes32) {
                return x;
            }
        }"#,
        |_| {},
    );
    let addr = src.contracts.last().unwrap();

    let payload = BytesN::<32>::from_array(
        &src.env,
        &[
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
            0xEE, 0xFF, 0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66, 0x55, 0x44,
            0x33, 0x22, 0x11, 0x00,
        ],
    );
    let res = src.invoke_contract(
        addr,
        "echo_bytes32",
        vec![payload.clone().into_val(&src.env)],
    );
    assert_eq!(BytesN::<32>::from_val(&src.env, &res), payload);
}
