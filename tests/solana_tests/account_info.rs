// SPDX-License-Identifier: Apache-2.0

use crate::{account_new, build_solidity, AccountState, BorshToken};
use num_bigint::BigInt;

#[test]
fn lamports() {
    let mut vm = build_solidity(
        r#"
        import 'solana';
        contract c {

            @mutableAccount(needle)
            function test() external payable returns (uint64) {
                AccountInfo ai = tx.accounts.needle;

                assert(ai.is_writable);
                assert(!ai.is_signer);
                assert(ai.executable);

                return ai.lamports;
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let acc = account_new();
    vm.account_data.insert(
        acc,
        AccountState {
            data: vec![],
            owner: None,
            lamports: 17672630920854456917u64,
        },
    );

    let returns = vm
        .function("test")
        .accounts(vec![("needle", acc)])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 64,
            value: BigInt::from(17672630920854456917u64),
        }
    );
}

#[test]
fn owner() {
    let mut vm = build_solidity(
        r#"
        import 'solana';
        contract c {
            function test() public payable returns (address) {
                return tx.accounts.dataAccount.owner;
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm
        .function("test")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

    let owner = vm.stack[0].id;

    assert_eq!(returns, BorshToken::Address(owner));
}

#[test]
fn data() {
    let mut vm = build_solidity(
        r#"
        import 'solana';
        contract c {
            function test(uint32 index) public payable returns (uint8) {
                return tx.accounts.dataAccount.data[index];
            }

            function test2() public payable returns (uint32, uint32) {
                AccountInfo ai = tx.accounts.dataAccount;
                return (ai.data.readUint32LE(1), ai.data.length);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    for i in 0..10 {
        let returns = vm
            .function("test")
            .arguments(&[BorshToken::Uint {
                width: 32,
                value: BigInt::from(i),
            }])
            .accounts(vec![("dataAccount", data_account)])
            .call()
            .unwrap();

        let val = vm.account_data[&data_account].data[i];

        assert_eq!(
            returns,
            BorshToken::Uint {
                width: 8,
                value: BigInt::from(val),
            }
        );
    }

    let returns = vm
        .function("test2")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

    let val = u32::from_le_bytes(
        vm.account_data[&data_account].data[1..5]
            .try_into()
            .unwrap(),
    );

    assert_eq!(
        returns,
        BorshToken::Tuple(vec![
            BorshToken::Uint {
                width: 32,
                value: BigInt::from(val),
            },
            BorshToken::Uint {
                width: 32,
                value: BigInt::from(4096),
            }
        ]),
    );
}

#[test]
fn modify_lamports() {
    let mut vm = build_solidity(
        r#"
import 'solana';

contract starter {

    @mutableAccount(acc1)
    @mutableAccount(acc2)
    @mutableAccount(acc3)
    function createNewAccount(uint64 lamport1, uint64 lamport2, uint64 lamport3) external {
        AccountInfo acc1 = tx.accounts.acc1;
        AccountInfo acc2 = tx.accounts.acc2;
        AccountInfo acc3 = tx.accounts.acc3;

        acc1.lamports -= lamport1;
        acc2.lamports = lamport2;
        acc3.lamports = acc3.lamports + lamport3;
    }
}
        "#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let acc1 = account_new();
    let acc2 = account_new();
    let acc3 = account_new();
    vm.account_data.insert(
        acc1,
        AccountState {
            data: vec![],
            owner: None,
            lamports: 25,
        },
    );
    vm.account_data.insert(
        acc2,
        AccountState {
            data: vec![],
            owner: None,
            lamports: 0,
        },
    );
    vm.account_data.insert(
        acc3,
        AccountState {
            data: vec![],
            owner: None,
            lamports: 2,
        },
    );

    let _ = vm
        .function("createNewAccount")
        .arguments(&[
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(20u8),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(7u8),
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(9u8),
            },
        ])
        .accounts(vec![("acc1", acc1), ("acc2", acc2), ("acc3", acc3)])
        .call();

    assert_eq!(vm.account_data.get(&acc1).unwrap().lamports, 5);
    assert_eq!(vm.account_data.get(&acc2).unwrap().lamports, 7);
    assert_eq!(vm.account_data.get(&acc3).unwrap().lamports, 11);
}

#[test]
fn account_data() {
    let mut vm = build_solidity(
        r#"
import 'solana';

contract C {

    @mutableAccount(acc)
	function test() external {
		AccountInfo ai = tx.accounts.acc;
		ai.data[0] = 0xca;
		ai.data[1] = 0xff;
		ai.data[2] = 0xee;
	}
}
        "#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let program_id = vm.stack[0].id;
    let other_account = account_new();
    vm.account_data.insert(
        other_account,
        AccountState {
            lamports: 0,
            owner: Some(program_id),
            data: vec![0; 3],
        },
    );

    vm.function("test")
        .accounts(vec![("acc", other_account)])
        .call();

    assert_eq!(vm.account_data[&other_account].data[0], 0xca);
    assert_eq!(vm.account_data[&other_account].data[1], 0xff);
    assert_eq!(vm.account_data[&other_account].data[2], 0xee);
}
