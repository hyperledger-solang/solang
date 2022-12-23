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
    vm.constructor(&[]);

    let returns = vm.function("f", &[]).unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8)
        },
    );

    let returns = vm.function("g", &[]).unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(3u8)
        },
    );

    let returns = vm.function("h", &[]).unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8)
        },
    );

    let returns = vm.function("i", &[]).unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(24u8)
        },
    );

    let returns = vm.function("j", &[]).unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(5u8)
        },
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

    vm.constructor(&[]);
    let returns = vm.function("f", &[]).unwrap().unwrap_tuple();

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

    vm.constructor(&[]);
    let returns = vm.function("f", &[]).unwrap().unwrap_tuple();

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

    vm.constructor(&[]);
    let _returns = vm.function("strange", &[]);
    let _returns = vm.function("inc", &[]);
    let returns = vm.function("get", &[]).unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8)
        },
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

    vm.constructor(&[]);
    let _returns = vm.function("f", &[]);
    let returns = vm.function("get", &[]).unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(5u8)
        },
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

    vm.constructor(&[]);
    let returns = vm.function("f", &[]).unwrap().unwrap_tuple();

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

    vm.constructor(&[]);
    let returns = vm.function("f", &[]).unwrap().unwrap_tuple();

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
