// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;

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

    vm.constructor(&[]);

    let returns = vm.function("f", &[]).unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(42u8)
        }
    );

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

    vm.constructor(&[]);

    let returns = vm.function("f", &[]).unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(42u8)
        }
    );
}
