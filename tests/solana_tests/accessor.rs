// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;
use num_traits::One;

#[test]
fn types() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            int64 public f1 = 102;
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("f1", &[], &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Int {
            width: 64,
            value: BigInt::from(102u8),
        }]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            int64[4] public f1 = [1,3,5,7];
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function(
        "f1",
        &[BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8),
        }],
        &[],
        None,
    );

    assert_eq!(
        returns,
        vec![BorshToken::Int {
            width: 64,
            value: BigInt::from(5u8)
        }]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            int64[4][2] public f1;

            constructor() {
                f1[1][0] = 4;
                f1[1][1] = 3;
                f1[1][2] = 2;
                f1[1][3] = 1;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function(
        "f1",
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::one(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(2u8),
            },
        ],
        &[],
        None,
    );

    assert_eq!(
        returns,
        vec![BorshToken::Int {
            width: 64,
            value: BigInt::from(2u8),
        }]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            mapping(int64 => uint64) public f1;

            constructor() {
                f1[2000] = 1;
                f1[4000] = 2;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function(
        "f1",
        &[BorshToken::Int {
            width: 64,
            value: BigInt::from(4000u16),
        }],
        &[],
        None,
    );

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 64,
            value: BigInt::from(2u8)
        }]
    );
}

#[test]
fn interfaces() {
    let mut vm = build_solidity(
        r#"
        contract foo is bar {
            bytes2 public f1 = "ab";
        }

        interface bar {
            function f1() external returns (bytes2);
        }
        "#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("f1", &[], &[], None);

    assert_eq!(returns, vec![BorshToken::FixedBytes(b"ab".to_vec())]);
}

#[test]
fn constant() {
    let mut vm = build_solidity(
        r#"
        contract x {
            bytes32 public constant z = keccak256("hey man");
        }"#,
    );

    vm.constructor("x", &[]);

    let returns = vm.function("z", &[], &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::FixedBytes(vec![
            0, 91, 121, 69, 17, 39, 209, 87, 169, 94, 81, 10, 68, 17, 183, 52, 82, 28, 128, 159,
            31, 73, 168, 235, 90, 61, 46, 198, 102, 241, 168, 79
        ])]
    );

    let mut vm = build_solidity(
        r#"
        contract x {
            bytes32 public constant z = sha256("hey man");
        }"#,
    );

    vm.constructor("x", &[]);

    let returns = vm.function("z", &[], &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::FixedBytes(vec![
            190, 212, 99, 127, 110, 196, 102, 135, 47, 156, 116, 193, 201, 43, 100, 230, 152, 184,
            58, 103, 63, 106, 217, 142, 143, 211, 220, 125, 255, 210, 48, 89
        ])]
    );

    let mut vm = build_solidity(
        r#"
        contract x {
            bytes20 public constant z = ripemd160("hey man");
        }"#,
    );

    vm.constructor("x", &[]);

    let returns = vm.function("z", &[], &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::FixedBytes(vec![
            255, 206, 178, 91, 165, 156, 178, 193, 7, 94, 233, 48, 117, 76, 48, 215, 255, 45, 61,
            225
        ])]
    );
}
