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

    vm.constructor("c", &[], 0);

    let returns = vm.function("test", &[], &[], 102);

    assert_eq!(returns[0], Token::Uint(ethereum_types::U256::from(306)));
}

#[test]
#[should_panic(expected = "4294967296")]
fn default_constructor_not_payable() {
    let mut vm = build_solidity(r#"contract c {}"#);

    vm.constructor("c", &[], 1);
}

#[test]
#[should_panic(expected = "4294967296")]
fn constructor_not_payable() {
    let mut vm = build_solidity(
        r#"
        contract c {
            constructor () {}
        }
    "#,
    );

    vm.constructor("c", &[], 1);
}

#[test]
#[should_panic(expected = "4294967296")]
fn function_not_payable() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function test() public {}
        }
    "#,
    );

    vm.constructor("c", &[], 0);

    vm.function("test", &[], &[], 102);
}
