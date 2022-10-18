// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;
use num_traits::One;

#[test]
fn return_single() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function f() public returns (uint) {
                return 2;
            }

            function g() public returns (uint) {
                return false? 2 : 3;
            }

            function h() public returns (uint) {
                return true? f() : g();
            }

            function i() public returns (uint) {
                int a = 24;
                return uint(a);
            }

            function j() public returns (uint) {
                return 2 + 3;
            }
        }"#,
    );
    vm.constructor_with_borsh("foo", &[]);

    let returns = vm.function_with_borsh("f", &[], &[], None);
    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8)
        },]
    );

    let returns = vm.function_with_borsh("g", &[], &[], None);
    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(3u8)
        },]
    );

    let returns = vm.function_with_borsh("h", &[], &[], None);
    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8)
        },]
    );

    let returns = vm.function_with_borsh("i", &[], &[], None);
    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(24u8)
        },]
    );

    let returns = vm.function_with_borsh("j", &[], &[], None);
    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(5u8)
        },]
    );
}

#[test]
fn return_ternary() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function f() public returns (uint, uint) {
                return true ? (false ? (1, 2) : (3, 4)) : (5, 6);
            }
        }"#,
    );

    vm.constructor_with_borsh("foo", &[]);
    let returns = vm.function_with_borsh("f", &[], &[], None);

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(3u8)
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(4u8)
            },
        ]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function f() public returns (uint, uint) {
                return true ? (1 + 2 + 3, 2 * 2) : (22 + 6, 1996);
            }
        }"#,
    );

    vm.constructor_with_borsh("foo", &[]);
    let returns = vm.function_with_borsh("f", &[], &[], None);

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(6u8)
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(4u8)
            },
        ]
    );
}

#[test]
fn return_nothing() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            uint private val = 0;

            function inc() public {
                val += 1;
            }

            function get() public returns (uint) {
                return val;
            }

            function strange() public {
                return inc();
            }

        }"#,
    );

    vm.constructor_with_borsh("foo", &[]);
    let _returns = vm.function_with_borsh("strange", &[], &[], None);
    let _returns = vm.function_with_borsh("inc", &[], &[], None);
    let returns = vm.function_with_borsh("get", &[], &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8)
        },]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            uint a = 4;

            function inc() internal {
                a += 1;
            }

            function dec() internal {
                a -= 1;
            }

            function get() public returns (uint) {
                return a;
            }

            function f() public {
                return true ? inc() : dec();
            }
        }"#,
    );

    vm.constructor_with_borsh("foo", &[]);
    let _returns = vm.function_with_borsh("f", &[], &[], None);
    let returns = vm.function_with_borsh("get", &[], &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(5u8)
        },]
    );
}

#[test]
fn return_function() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function g() public returns (uint, uint) {
                return (1, 2);
            }

            function f() public returns (uint, uint) {
                return g();
            }
        }"#,
    );

    vm.constructor_with_borsh("foo", &[]);
    let returns = vm.function_with_borsh("f", &[], &[], None);

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 256,
                value: BigInt::one()
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(2u8)
            },
        ]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function g() public returns (uint, uint) {
                return (1, 2);
            }

            function f() public returns (uint, uint) {
                return true? g() : (0, 0);
            }
        }"#,
    );

    vm.constructor_with_borsh("foo", &[]);
    let returns = vm.function_with_borsh("f", &[], &[], None);

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 256,
                value: BigInt::one()
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(2u8)
            },
        ]
    );
}
