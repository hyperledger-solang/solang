// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
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

            function test2() public payable returns (uint32) {
                for (uint32 i = 0; i < tx.accounts.length; i++) {
                    AccountInfo ai = tx.accounts[i];

                    if (ai.key == address(this)) {
                        return ai.data.readUint32LE(1);
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
        BorshToken::Uint {
            width: 32,
            value: BigInt::from(val),
        }
    );
}
