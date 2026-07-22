// SPDX-License-Identifier: Apache-2.0
use crate::build_solidity;
use soroban_sdk::{Bytes, BytesN, IntoVal, TryFromVal};

#[test]
fn sha256_basic() {
    let runtime = build_solidity(
        r#"contract HashTest {
            function hash(bytes memory input) public pure returns (bytes32) {
                return sha256(input);
            }
        }"#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let input = Bytes::from_slice(&runtime.env, b"abc");
    let result = runtime.invoke_contract(addr, "hash", vec![input.into_val(&runtime.env)]);

    let result_bytes = BytesN::<32>::try_from_val(&runtime.env, &result).unwrap();

    let expected = BytesN::<32>::from_array(
        &runtime.env,
        &[
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ],
    );

    assert_eq!(result_bytes, expected);
}
