use crate::{build_solidity, first_error, parse_and_resolve, Target};
use ethabi::Token;

#[test]
fn library_constant() {
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

    let returns = vm.function("f", &[], &[], 0);

    assert_eq!(returns, vec![Token::Uint(ethereum_types::U256::from(42))]);
}

#[test]
fn contract_constant() {
    let ns = parse_and_resolve(
        r#"
        contract Library {
            uint256 public constant STATIC = 42;
        }

        contract foo {
            function f() public returns (uint) {
                uint a = Library.STATIC;
                return a;
            }
        }
        "#,
        Target::Solana,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "conversion from function() internal view returns (uint256) to uint256 not possible",
    );

    let ns = parse_and_resolve(
        r#"
        contract Library {
            uint256 public constant STATIC = 42;
        }

        contract foo {
            function f() public returns (uint) {
                uint a = Library.STATIC();
                return a;
            }
        }
        "#,
        Target::Solana,
    );

    assert_eq!(first_error(ns.diagnostics), "`Library' is a contract",);
}
