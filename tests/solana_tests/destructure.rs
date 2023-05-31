// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;
use num_traits::One;

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

    let returns = vm
        .function("f", &[BorshToken::Bool(true), BorshToken::Bool(true)])
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Int {
                width: 256,
                value: BigInt::one(),
            },
            BorshToken::Int {
                width: 256,
                value: BigInt::from(2u8),
            }
        ]
    );

    let returns = vm
        .function("f", &[BorshToken::Bool(true), BorshToken::Bool(false)])
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Int {
                width: 256,
                value: BigInt::from(3u8),
            },
            BorshToken::Int {
                width: 256,
                value: BigInt::from(4u8),
            }
        ]
    );

    let returns = vm
        .function("f", &[BorshToken::Bool(false), BorshToken::Bool(false)])
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Int {
                width: 256,
                value: BigInt::from(5u8),
            },
            BorshToken::Int {
                width: 256,
                value: BigInt::from(6u8),
            }
        ]
    );

    let returns = vm
        .function("f", &[BorshToken::Bool(false), BorshToken::Bool(true)])
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Int {
                width: 256,
                value: BigInt::from(5u8),
            },
            BorshToken::Int {
                width: 256,
                value: BigInt::from(6u8),
            }
        ]
    );
}

#[test]
fn casting_destructure() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            int[] arr;
            function f() public returns (int, int) {
                int[] storage ptrArr = arr;
                ptrArr.push(1);
                ptrArr.push(2);
                (int a, int b) = (ptrArr[0], ptrArr[1]);
                return (a, b);
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("f", &[]).unwrap().unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Int {
                width: 256,
                value: BigInt::one(),
            },
            BorshToken::Int {
                width: 256,
                value: BigInt::from(2u8),
            }
        ]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function f() public returns (string) {
                (string a, string b) = ("Hello", "World!");
                return (a);
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("f", &[]).unwrap();

    assert_eq!(returns, BorshToken::String(String::from("Hello")));
}

#[test]
fn casting_storage_destructure() {
    let mut vm = build_solidity(
        r#"
        contract c {
            address factory;
            int decimals;
            int[2] arr1;
            int[2] arr2;

            constructor() {
                int[2] storage x;

                (x, factory, decimals) = foo();
                x[0] = 2;
            }

            function foo() internal view returns (int[2] storage, address, int) {
                return (arr2, address(2), 5);
            }

            function bar() public view {
                require(factory == address(2), "address wrong");
                require(decimals == 5, "int wrong");
                require(arr2[0] == 2, "array wrong");
            }
        }"#,
    );

    vm.constructor(&[]);

    vm.function("bar", &[]);
}
