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
    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm.function("f").call().unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8)
        },
    );

    let returns = vm.function("g").call().unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(3u8)
        },
    );

    let returns = vm.function("h").call().unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8)
        },
    );

    let returns = vm.function("i").call().unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(24u8)
        },
    );

    let returns = vm.function("j").call().unwrap();
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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let returns = vm.function("f").call().unwrap().unwrap_tuple();

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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let returns = vm.function("f").call().unwrap().unwrap_tuple();

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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let _returns = vm
        .function("strange")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let _returns = vm
        .function("inc")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let returns = vm
        .function("get")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let _returns = vm
        .function("f")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let returns = vm
        .function("get")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let returns = vm.function("f").call().unwrap().unwrap_tuple();

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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let returns = vm.function("f").call().unwrap().unwrap_tuple();

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
