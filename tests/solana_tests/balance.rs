use crate::build_solidity;
use ethabi::Token;

#[test]
fn msg_value() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function test() public payable returns (uint64) {
                return msg.value * 3;
            }
        }"#,
    );

    vm.constructor("c", &[]);

    let returns = vm.function("test", &[], &[], 102);

    assert_eq!(returns[0], Token::Uint(ethereum_types::U256::from(306)));
}
