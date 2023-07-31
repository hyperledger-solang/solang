// SPDX-License-Identifier: Apache-2.0

use crate::{account_new, build_solidity, AccountMeta, AccountState, BorshToken, Pubkey};

#[test]
fn use_authority() {
    let mut vm = build_solidity(include_str!("../../docs/examples/solana/use_authority.sol"));

    let authority = account_new();

    vm.account_data.insert(
        authority,
        AccountState {
            data: vec![],
            owner: Some([0u8; 32]),
            lamports: 0,
        },
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .arguments(&[BorshToken::Address(authority)])
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let res = vm
        .function("inc")
        .accounts(vec![("dataAccount", data_account)])
        .must_fail()
        .unwrap();
    assert_ne!(res, 0);

    let res = vm
        .function("get")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();
    assert_eq!(
        res,
        BorshToken::Uint {
            width: 64,
            value: 0.into()
        }
    );

    vm.function("inc")
        .accounts(vec![("dataAccount", data_account)])
        .remaining_accounts(&[AccountMeta {
            pubkey: Pubkey(authority),
            is_signer: true,
            is_writable: false,
        }])
        .call();

    let res = vm
        .function("get")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();
    assert_eq!(
        res,
        BorshToken::Uint {
            width: 64,
            value: 1.into()
        }
    );
}
