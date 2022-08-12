// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use ethabi::{encode, ethereum_types::U256, Token};

#[test]
fn abi_encode() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (bytes) {
                return abi.encode(true, "foobar");
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("test", &[]);

    let bytes = encode(&[Token::Bool(true), Token::String(String::from("foobar"))]);

    assert_eq!(returns, vec![Token::Bytes(bytes)]);

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (bytes) {
                bytes4 h = "ABCD";
                return abi.encodeWithSelector(0x04030201, int(102), h);
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("test", &[]);

    let mut bytes = vec![4, 3, 2, 1];

    bytes.extend(
        encode(&[
            Token::Int(U256::from(102)),
            Token::FixedBytes(b"ABCD".to_vec()),
        ])
        .iter(),
    );

    assert_eq!(returns, vec![Token::Bytes(bytes)]);
}

#[test]
fn selector_override() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public selector=hex"00000001" returns (uint) {
                return 102;
            }
        }"#,
    );

    vm.constructor(&[]);

    vm.raw_function(vec![0, 0, 0, 1]);

    let returns = vm.abi.functions["test"][0]
        .decode_output(&vm.vm.output)
        .unwrap();

    assert_eq!(returns, vec![Token::Uint(U256::from(102))]);
}

#[test]
#[should_panic(expected = "fail to invoke main: revert")]
fn cannot_use_old_selector() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public selector=hex"00000001" returns (uint) {
                return 102;
            }
        }"#,
    );

    vm.constructor(&[]);

    vm.function_abi_fail("test", &[], |_| ());
}
