// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use base58::{FromBase58, ToBase58};
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
            function sig() public returns (bytes8) {
                return msg.sig;
            }
            function prog() public returns (address) {
                return address(this);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let clock_account = <[u8; 32]>::try_from(
        "SysvarC1ock11111111111111111111111111111111"
            .from_base58()
            .unwrap(),
    )
    .unwrap();
    let returns = vm
        .function("mr_now")
        .accounts(vec![("clock", clock_account)])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 64,
            value: BigInt::from(1620656423u64)
        }
    );

    let returns = vm
        .function("mr_slot")
        .accounts(vec![("clock", clock_account)])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 64,
            value: BigInt::from(70818331u64),
        }
    );

    let returns = vm
        .function("mr_blocknumber")
        .accounts(vec![("clock", clock_account)])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 64,
            value: BigInt::from(70818331u64)
        },
    );

    let returns = vm
        .function("msg_data")
        .arguments(&[BorshToken::Uint {
            width: 32,
            value: BigInt::from(0xdeadcafeu32),
        }])
        .call()
        .unwrap();

    if let BorshToken::Bytes(v) = &returns {
        println!("{}", hex::encode(v));
    }

    assert_eq!(
        returns,
        BorshToken::Bytes(hex::decode("a73fcaa3b216e85afecaadde").unwrap())
    );

    let returns = vm.function("sig").call().unwrap();

    if let Some(v) = returns.clone().into_fixed_bytes() {
        println!("{}", hex::encode(v));
    }

    assert_eq!(
        returns,
        BorshToken::uint8_fixed_array(hex::decode("4b22101a3c98d6cb").unwrap())
    );

    let returns = vm.function("prog").call().unwrap();

    assert_eq!(returns, BorshToken::Address(vm.stack[0].id));
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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm
        .function("create_pda")
        .arguments(&[BorshToken::Bool(true)])
        .accounts(vec![("systemProgram", [0; 32])])
        .call()
        .unwrap();

    if let Some(bs) = returns.clone().into_fixed_bytes() {
        assert_eq!(
            bs.to_base58(),
            "2fnQrngrQT4SeLcdToJAD96phoEjNL2man2kfRLCASVk"
        );
    } else {
        panic!("{returns:?} not expected");
    }

    let returns = vm
        .function("create_pda")
        .arguments(&[BorshToken::Bool(false)])
        .accounts(vec![("systemProgram", [0; 32])])
        .call()
        .unwrap();

    if let Some(bs) = returns.clone().into_fixed_bytes() {
        assert_eq!(
            bs.to_base58(),
            "7YgSsrAiAEJFqBNujFBRsEossqdpV31byeJLBsZ5QSJE"
        );
    } else {
        panic!("{returns:?} not expected");
    }

    let returns = vm
        .function("create_pda2")
        .arguments(&[
            BorshToken::Bytes(b"Talking".to_vec()),
            BorshToken::Bytes(b"Squirrels".to_vec()),
        ])
        .accounts(vec![("systemProgram", [0; 32])])
        .call()
        .unwrap();

    if let Some(bs) = returns.clone().into_fixed_bytes() {
        assert_eq!(
            bs.to_base58(),
            "2fnQrngrQT4SeLcdToJAD96phoEjNL2man2kfRLCASVk"
        );
    } else {
        panic!("{returns:?} not expected");
    }

    let returns = vm
        .function("create_pda2_bump")
        .arguments(&[BorshToken::Bool(true)])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(returns[1], BorshToken::uint8_fixed_array(vec![255]));

    if let Some(bs) = returns[0].clone().into_fixed_bytes() {
        assert_eq!(
            bs.to_base58(),
            "DZpR2BwsPVtbXxUUbMx5tK58Ln2T9RUtAshtR2ePqDcu"
        );
    } else {
        panic!("{returns:?} not expected");
    }

    let returns = vm
        .function("create_pda2_bump")
        .arguments(&[BorshToken::Bool(false)])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(returns[1], BorshToken::uint8_fixed_array(vec![255]));

    if let Some(bs) = returns[0].clone().into_fixed_bytes() {
        assert_eq!(
            bs.to_base58(),
            "3Y19WiAiLD8kT8APmtk41NgHEpkYTzx28s1uwAX8LJq4"
        );
    } else {
        panic!("{returns:?} not expected");
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
    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let returns = vm.function("testStringAndBytes").call().unwrap();
    let bytes = returns.into_bytes().unwrap();

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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let _ = vm.function("testBytesOut").call();
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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let _ = vm.function("testStringOut").call();
}
