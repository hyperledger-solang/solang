// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;
use rand::Rng;

#[test]
fn interfaceid() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function get() public returns (bytes8) {
                return type(I).interfaceId;
            }
        }

        interface I {
            function bar(int) external;
            function baz(bytes) external returns (int);
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("get", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::FixedBytes(
            0x88632631fac67239u64.to_be_bytes().to_vec()
        )]
    );
}

#[test]
fn write_buffer() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test1() public returns (bytes) {
                bytes bs = new bytes(12);
                bs.writeInt32LE(-0x41424344, 0);
                bs.writeUint64LE(0x0102030405060708, 4);
                return bs;
            }

            function test2() public returns (bytes) {
                bytes bs = new bytes(34);
                bs.writeUint16LE(0x4142, 0);
                bs.writeAddress(msg.sender, 2);
                return bs;
            }

            function test3() public returns (bytes) {
                bytes bs = new bytes(9);
                bs.writeUint64LE(1, 2);
                return bs;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("test1", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Bytes(
            [0xbc, 0xbc, 0xbd, 0xbe, 8, 7, 6, 5, 4, 3, 2, 1].to_vec()
        )]
    );

    let returns = vm.function("test2", &[], None);

    let mut buf = vec![0x42u8, 0x41u8];
    buf.extend_from_slice(&vm.origin);

    assert_eq!(returns, vec![BorshToken::Bytes(buf)]);

    let res = vm.function_must_fail("test3", &[], None);
    assert_eq!(res, Ok(4294967296));
}

#[test]
fn read_buffer() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test1(bytes bs) public returns (int32, uint64) {
                return (bs.readInt32LE(0), bs.readUint64LE(4));
            }

            function test2(bytes bs) public returns (uint16, address) {
                return (bs.readUint16LE(0), bs.readAddress(2));
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function(
        "test1",
        &[BorshToken::Bytes(
            [0xbc, 0xbc, 0xbd, 0xbe, 8, 7, 6, 5, 4, 3, 2, 1].to_vec(),
        )],
        None,
    );

    assert_eq!(
        returns,
        vec![
            BorshToken::Int {
                width: 32,
                value: BigInt::from(-1094861636),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(0x0102030405060708u64),
            },
        ]
    );

    let res = vm.function_must_fail(
        "test1",
        &[BorshToken::Bytes(
            [0xbc, 0xbc, 0xbd, 0xbe, 8, 7, 6, 5, 4, 3, 2].to_vec(),
        )],
        None,
    );
    assert_eq!(res, Ok(4294967296));

    let mut buf = vec![0x42u8, 0x41u8];
    buf.extend_from_slice(&vm.origin);

    let returns = vm.function("test2", &[BorshToken::Bytes(buf.clone())], None);

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 16,
                value: BigInt::from(0x4142u16)
            },
            BorshToken::FixedBytes(vm.origin.to_vec())
        ]
    );

    buf.pop();

    let res = vm.function_must_fail("test2", &[BorshToken::Bytes(buf)], None);
    assert_eq!(res, Ok(4294967296));
}

#[test]
fn bytes_compare() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test1(bytes4 bs) public returns (bool) {
                return bs != 0;
            }

            function test2(bytes4 bs) public returns (bool) {
                return bs == 0;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function(
        "test1",
        &[BorshToken::FixedBytes([0xbc, 0xbc, 0xbd, 0xbe].to_vec())],
        None,
    );

    assert_eq!(returns, vec![BorshToken::Bool(true)]);

    let returns = vm.function(
        "test2",
        &[BorshToken::FixedBytes([0xbc, 0xbc, 0xbd, 0xbe].to_vec())],
        None,
    );

    assert_eq!(returns, vec![BorshToken::Bool(false)]);
}

#[test]
fn assignment_in_ternary() {
    let mut rng = rand::thread_rng();

    let mut vm = build_solidity(
        r#"
        contract foo {
            function minimum(uint64 x, uint64 y) public pure returns (uint64 z) {
                x >= y ? z = y : z = x;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    for _ in 0..10 {
        let left = rng.gen::<u64>();
        let right = rng.gen::<u64>();

        let returns = vm.function(
            "minimum",
            &[
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(left),
                },
                BorshToken::Uint {
                    width: 64,
                    value: BigInt::from(right),
                },
            ],
            None,
        );

        assert_eq!(
            returns,
            vec![BorshToken::Uint {
                width: 64,
                value: BigInt::from(std::cmp::min(left, right))
            },]
        );
    }
}

#[test]
fn power() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function power() public returns (uint) {
                return 2 ** 3 ** 4;
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("power", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 256,
            value: BigInt::from(2417851639229258349412352u128)
        },]
    );
}
