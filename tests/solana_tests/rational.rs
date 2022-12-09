// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;
use num_traits::Zero;

#[test]
fn rational() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = .5 * 8;
                return x;
            }

            function test2() public returns (uint) {
                uint x = .4 * 8 + 0.8;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(4u8)
        }]
    );

    let returns = vm.function("test2", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(4u8)
        }]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = 4.8 + 0.2;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(5u8)
        }]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = 4.8 / 0.2;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(24)
        }]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = 4.8 % 0.2;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::zero(),
        }]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                uint x = 5.2 - 1.2;
                return x;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(4u8),
        }]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                return 1.4 + 1.6;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(3u8)
        }]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (uint) {
                return 1.4e4 + 1.6e3;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(15600u32)
        }]
    );

    let mut vm = build_solidity(
        r#"
        contract foo {
            function test(uint64 x) public returns (uint64, uint) {
                return (x * 961748941, 2.5 + 3.5 - 1);
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function(
        "test",
        &[BorshToken::Uint {
            width: 64,
            value: BigInt::from(982451653u32),
        }],
        None,
    );

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(961748941u64 * 982451653u64)
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(5u8)
            },
        ]
    );
}
