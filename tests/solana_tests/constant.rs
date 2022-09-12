// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use ethabi::{ethereum_types::U256, Token};

#[test]
fn constant_in_library() {
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

    vm.constructor("foo", &[]);

    let returns = vm.function("f", &[], &[], None);
    assert_eq!(returns, vec![Token::Uint(U256::from(42))]);
}

#[test]
#[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
fn constant_in_contract() {
    build_solidity(
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
}
