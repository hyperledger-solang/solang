// SPDX-License-Identifier: Apache-2.0

use crate::{account_new, build_solidity, AccountMeta, AccountState, BorshToken, Pubkey};
use num_bigint::BigInt;

#[test]
fn lamports() {
    let mut vm = build_solidity(
        r#"
        import 'solana';
        contract c {
            function test(address needle) public payable returns (uint64) {
                for (uint32 i = 0; i < tx.accounts.length; i++) {
                    AccountInfo ai = tx.accounts[i];

                    assert(ai.is_writable);
                    assert(!ai.is_signer);
                    assert(ai.executable);

                    if (ai.key == needle) {
                        return ai.lamports;
                    }
                }

                revert("account not found");
            }
        }"#,
    );

    vm.constructor(&[]);

    vm.account_data.get_mut(&vm.origin).unwrap().lamports = 17672630920854456917u64;

    let returns = vm
        .function("test", &[BorshToken::Address(vm.origin)])
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
                for (uint32 i = 0; i < tx.accounts.length; i++) {
                    AccountInfo ai = tx.accounts[i];

                    if (ai.key == address(this)) {
                        return ai.owner;
                    }
                }

                revert("account not found");
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("test", &[]).unwrap();

    let owner = vm.stack[0].program;

    assert_eq!(returns, BorshToken::Address(owner));
}

#[test]
fn data() {
    let mut vm = build_solidity(
        r#"
        import 'solana';
        contract c {
            function test(uint32 index) public payable returns (uint8) {
                for (uint32 i = 0; i < tx.accounts.length; i++) {
                    AccountInfo ai = tx.accounts[i];

                    if (ai.key == address(this)) {
                        return ai.data[index];
                    }
                }

                revert("account not found");
            }

            function test2() public payable returns (uint32, uint32) {
                for (uint32 i = 0; i < tx.accounts.length; i++) {
                    AccountInfo ai = tx.accounts[i];

                    if (ai.key == address(this)) {
                        return (ai.data.readUint32LE(1), ai.data.length);
                    }
                }

                revert("account not found");
            }
        }"#,
    );

    vm.constructor(&[]);

    for i in 0..10 {
        let returns = vm
            .function(
                "test",
                &[BorshToken::Uint {
                    width: 32,
                    value: BigInt::from(i),
                }],
            )
            .unwrap();

        let this = &vm.stack[0].data;

        let val = vm.account_data[this].data[i];

        assert_eq!(
            returns,
            BorshToken::Uint {
                width: 8,
                value: BigInt::from(val),
            }
        );
    }

    let returns = vm.function("test2", &[]).unwrap();

    let this = &vm.stack[0].data;

    let val = u32::from_le_bytes(vm.account_data[this].data[1..5].try_into().unwrap());

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
    function createNewAccount(uint64 lamport1, uint64 lamport2, uint64 lamport3) public {
        AccountInfo acc1 = tx.accounts[1];
        AccountInfo acc2 = tx.accounts[2];
        AccountInfo acc3 = tx.accounts[3];

        acc1.lamports -= lamport1;
        acc2.lamports = lamport2;
        acc3.lamports = acc3.lamports + lamport3;
    }
}
        "#,
    );

    vm.constructor(&[]);

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

    let metas = vec![
        AccountMeta {
            pubkey: Pubkey(vm.stack[0].data),
            is_writable: false,
            is_signer: false,
        },
        AccountMeta {
            pubkey: Pubkey(acc1),
            is_writable: true,
            is_signer: false,
        },
        AccountMeta {
            pubkey: Pubkey(acc2),
            is_writable: true,
            is_signer: false,
        },
        AccountMeta {
            pubkey: Pubkey(acc3),
            is_writable: true,
            is_signer: false,
        },
    ];

    let _ = vm.function_metas(
        "createNewAccount",
        &metas,
        &[
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
        ],
    );

    assert_eq!(vm.account_data.get(&acc1).unwrap().lamports, 5);
    assert_eq!(vm.account_data.get(&acc2).unwrap().lamports, 7);
    assert_eq!(vm.account_data.get(&acc3).unwrap().lamports, 11);
}
