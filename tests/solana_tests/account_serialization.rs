// SPDX-License-Identifier: Apache-2.0

use crate::borsh_encoding::BorshToken;
use crate::{account_new, build_solidity, AccountMeta, AccountState, Pubkey};

#[test]
fn deserialize_duplicate_account() {
    let mut vm = build_solidity(
        r#"
        contract Testing {
    function check_deserialization(address my_address) public view {
        assert(tx.accounts[1].key == tx.accounts[2].key);
        assert(tx.accounts[1].is_signer == tx.accounts[2].is_signer);
        assert(tx.accounts[1].is_writable == tx.accounts[2].is_writable);

        assert(my_address == tx.accounts.dataAccount.key);
    }
}
        "#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let random_key = account_new();
    vm.account_data.insert(
        random_key,
        AccountState {
            data: vec![],
            owner: None,
            lamports: 0,
        },
    );

    let other_key = account_new();
    vm.account_data.insert(
        other_key,
        AccountState {
            data: vec![],
            owner: None,
            lamports: 0,
        },
    );

    vm.function("check_deserialization")
        .arguments(&[BorshToken::Address(data_account)])
        .accounts(vec![("dataAccount", data_account)])
        .remaining_accounts(&[
            AccountMeta {
                pubkey: Pubkey(random_key),
                is_signer: true,
                is_writable: false,
            },
            AccountMeta {
                pubkey: Pubkey(random_key),
                is_signer: true,
                is_writable: false,
            },
        ])
        .call();
}

#[test]
fn more_than_10_accounts() {
    let mut vm = build_solidity(
        r#"
        contract Testing {
    function check_deserialization(address my_address) public view {
        // This assertion ensure the padding is correctly added when
        // deserializing accounts
        assert(my_address == address(this));
    }
}
        "#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let mut metas: Vec<AccountMeta> = Vec::new();
    for i in 0..11 {
        let account = account_new();
        metas.push(AccountMeta {
            pubkey: Pubkey(account),
            is_writable: i % 2 == 0,
            is_signer: i % 2 == 1,
        });
        vm.account_data.insert(
            account,
            AccountState {
                data: vec![],
                owner: None,
                lamports: 0,
            },
        );
    }

    metas.push(metas[3].clone());
    let account = account_new();
    metas.push(AccountMeta {
        pubkey: Pubkey(account),
        is_signer: false,
        is_writable: false,
    });
    vm.account_data.insert(
        account,
        AccountState {
            data: vec![],
            owner: None,
            lamports: 0,
        },
    );

    let program_id = vm.stack[0].id;
    vm.function("check_deserialization")
        .arguments(&[BorshToken::Address(program_id)])
        .remaining_accounts(&metas)
        .call();
}
