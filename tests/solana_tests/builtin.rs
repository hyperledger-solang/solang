// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use base58::ToBase58;
use num_bigint::BigInt;

#[test]
fn builtins() {
    let mut vm = build_solidity(
        r#"
        contract timestamp {
            function mr_now() public returns (uint64) {
                return block.timestamp;
            }
            function mr_slot() public returns (uint64) {
                return block.slot;
            }
            function mr_blocknumber() public returns (uint64) {
                return block.number;
            }
            function msg_data(uint32 x) public returns (bytes) {
                return msg.data;
            }
            function sig() public returns (bytes4) {
                return msg.sig;
            }
            function prog() public returns (address) {
                return tx.program_id;
            }
        }"#,
    );

    vm.constructor("timestamp", &[]);

    let returns = vm.function("mr_now", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 64,
            value: BigInt::from(1620656423u64)
        }]
    );

    let returns = vm.function("mr_slot", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 64,
            value: BigInt::from(70818331u64),
        }]
    );

    let returns = vm.function("mr_blocknumber", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Uint {
            width: 64,
            value: BigInt::from(70818331u64)
        },]
    );

    let returns = vm.function(
        "msg_data",
        &[BorshToken::Uint {
            width: 32,
            value: BigInt::from(0xdeadcafeu32),
        }],
        None,
    );

    if let BorshToken::Bytes(v) = &returns[0] {
        println!("{}", hex::encode(v));
    }

    assert_eq!(
        returns,
        vec![BorshToken::Bytes(hex::decode("84da38e0fecaadde").unwrap())]
    );

    let returns = vm.function("sig", &[], None);

    if let BorshToken::FixedBytes(v) = &returns[0] {
        println!("{}", hex::encode(v));
    }

    assert_eq!(
        returns,
        vec![BorshToken::FixedBytes(hex::decode("00a7029b").unwrap())]
    );

    let returns = vm.function("prog", &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::FixedBytes(vm.stack[0].program.to_vec())]
    );
}

#[test]
fn pda() {
    let mut vm = build_solidity(
        r#"
        import 'solana';

        contract pda {
            function create_pda(bool cond) public returns (address) {
                address program_id = address"BPFLoaderUpgradeab1e11111111111111111111111";
                address addr = create_program_address(["Talking", "Cats"], program_id);
                if (cond) {
                    return create_program_address(["Talking", "Squirrels"], program_id);
                } else {
                    return addr;
                }
            }

            function create_pda2(bytes a, bytes b) public returns (address) {
                address program_id = address"BPFLoaderUpgradeab1e11111111111111111111111";

                return create_program_address([a, b], program_id);
            }

            function create_pda2_bump(bool cond) public returns (address, bytes1) {
                address program_id = address"BPFLoaderUpgradeab1e11111111111111111111111";
                (address addr, bytes1 bump) = try_find_program_address(["bar", hex"01234567"], program_id);

                if (cond) {
                    return try_find_program_address(["foo", hex"01234567"], program_id);
                } else {
                    return (addr, bump);
                }
            }
        }"#,
    );

    vm.constructor("pda", &[]);

    let returns = vm.function("create_pda", &[BorshToken::Bool(true)], None);

    if let BorshToken::FixedBytes(bs) = &returns[0] {
        assert_eq!(
            bs.to_base58(),
            "2fnQrngrQT4SeLcdToJAD96phoEjNL2man2kfRLCASVk"
        );
    } else {
        panic!("{:?} not expected", returns);
    }

    let returns = vm.function("create_pda", &[BorshToken::Bool(false)], None);

    if let BorshToken::FixedBytes(bs) = &returns[0] {
        assert_eq!(
            bs.to_base58(),
            "7YgSsrAiAEJFqBNujFBRsEossqdpV31byeJLBsZ5QSJE"
        );
    } else {
        panic!("{:?} not expected", returns);
    }

    let returns = vm.function(
        "create_pda2",
        &[
            BorshToken::Bytes(b"Talking".to_vec()),
            BorshToken::Bytes(b"Squirrels".to_vec()),
        ],
        None,
    );

    if let BorshToken::FixedBytes(bs) = &returns[0] {
        assert_eq!(
            bs.to_base58(),
            "2fnQrngrQT4SeLcdToJAD96phoEjNL2man2kfRLCASVk"
        );
    } else {
        panic!("{:?} not expected", returns);
    }

    let returns = vm.function("create_pda2_bump", &[BorshToken::Bool(true)], None);

    assert_eq!(returns[1], BorshToken::FixedBytes(vec![255]));

    if let BorshToken::FixedBytes(bs) = &returns[0] {
        assert_eq!(
            bs.to_base58(),
            "DZpR2BwsPVtbXxUUbMx5tK58Ln2T9RUtAshtR2ePqDcu"
        );
    } else {
        panic!("{:?} not expected", returns);
    }

    let returns = vm.function("create_pda2_bump", &[BorshToken::Bool(false)], None);

    assert_eq!(returns[1], BorshToken::FixedBytes(vec![255]));

    if let BorshToken::FixedBytes(bs) = &returns[0] {
        assert_eq!(
            bs.to_base58(),
            "3Y19WiAiLD8kT8APmtk41NgHEpkYTzx28s1uwAX8LJq4"
        );
    } else {
        panic!("{:?} not expected", returns);
    }
}

#[test]
fn test_string_bytes_buffer_write() {
    let mut vm = build_solidity(
        r#"
    contract Testing {
        function testStringAndBytes() public pure returns (bytes memory) {
            string str = "coffee";
            bytes memory b = new bytes(9);
            b.writeString(str, 0);
            bytes memory g = "tea";
            b.writeBytes(g, 6);
            return b;
        }
    }
        "#,
    );
    vm.constructor("Testing", &[]);
    let returns = vm.function("testStringAndBytes", &[], None);
    let bytes = returns[0].clone().into_bytes().unwrap();

    assert_eq!(bytes.len(), 9);
    assert_eq!(&bytes[0..6], b"coffee");
    assert_eq!(&bytes[6..9], b"tea");
}

#[test]
#[should_panic(expected = "unexpected return 0x100000000")]
fn out_of_bounds_bytes_write() {
    let mut vm = build_solidity(
        r#"
    contract Testing {
        function testBytesOut() public pure returns (bytes memory) {
            bytes memory b = new bytes(9);
            bytes memory g = "tea";
            b.writeBytes(g, 30);
            return b;
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let _ = vm.function("testBytesOut", &[], None);
}

#[test]
#[should_panic(expected = "unexpected return 0x100000000")]
fn out_of_bounds_string_write() {
    let mut vm = build_solidity(
        r#"
    contract Testing {
        function testStringOut() public pure returns (bytes memory) {
            bytes memory b = new bytes(4);
            string memory str = "cappuccino";
            b.writeString(str, 0);
            return b;
        }
    }
        "#,
    );

    vm.constructor("Testing", &[]);
    let _ = vm.function("testStringOut", &[], None);
}
