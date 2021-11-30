use crate::build_solidity;
use ethabi::Token;

#[test]
fn constant() {
    let mut vm = build_solidity(
        r#"
        library Library {
            uint256 internal constant STATIC = 42;
        }

        contract foo {
            function f() public returns (uint) {
                uint a = Library.STATIC;
                return a;
            }
        }
        "#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function("f", &[], &[], 0, None);
    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(42))]);

    let mut vm = build_solidity(
        r#"
        contract C {
            uint256 public constant STATIC = 42;
        }

        contract foo {
            function f() public returns (uint) {
                uint a = C.STATIC;
                return a;
            }
        }
        "#,
    );

    vm.constructor("foo", &[], 0);

    let returns = vm.function("f", &[], &[], 0, None);
    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(42))]);
}
