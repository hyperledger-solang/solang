use crate::build_solidity;
use ethabi::Token;

#[test]
fn conditional_destructure() {
    // test that the abi encoder can handle fixed arrays
    let mut vm = build_solidity(
        r#"
        contract foo {
            function f(bool cond1, bool cond2) public returns (int, int) {
                (int a, int b) = cond1 ? (cond2 ? (1, 2) : (3, 4)) : (5, 6);

                return (a, b);
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("f", &[Token::Bool(true), Token::Bool(true)]);

    assert_eq!(
        returns,
        vec![
            Token::Int(ethereum_types::U256::from(1)),
            Token::Int(ethereum_types::U256::from(2)),
        ]
    );

    let returns = vm.function("f", &[Token::Bool(true), Token::Bool(false)]);

    assert_eq!(
        returns,
        vec![
            Token::Int(ethereum_types::U256::from(3)),
            Token::Int(ethereum_types::U256::from(4)),
        ]
    );

    let returns = vm.function("f", &[Token::Bool(false), Token::Bool(false)]);

    assert_eq!(
        returns,
        vec![
            Token::Int(ethereum_types::U256::from(5)),
            Token::Int(ethereum_types::U256::from(6)),
        ]
    );

    let returns = vm.function("f", &[Token::Bool(false), Token::Bool(true)]);

    assert_eq!(
        returns,
        vec![
            Token::Int(ethereum_types::U256::from(5)),
            Token::Int(ethereum_types::U256::from(6)),
        ]
    );
}
