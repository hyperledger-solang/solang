use crate::build_solidity;
use ethabi::{encode, Token};

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

    let mut bytes = vec![1, 2, 3, 4];

    bytes.extend(
        encode(&[
            Token::Int(ethereum_types::U256::from(102)),
            Token::FixedBytes(b"ABCD".to_vec()),
        ])
        .iter(),
    );

    assert_eq!(returns, vec![Token::Bytes(bytes)]);
}
