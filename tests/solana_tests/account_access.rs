// SPDX-License-Identifier: Apache-2.0

use crate::borsh_encoding::BorshToken;
use crate::{account_new, build_solidity, AccountMeta, AccountState, Pubkey};

#[test]
fn access_payer() {
    let mut vm = build_solidity(
        r#"
        contract Test {
    @payer(payer)
    @seed("sunflower")
    @space(23)
    constructor(address my_payer) {
        assert(tx.accounts.payer.key == my_payer);
        assert(tx.accounts.payer.is_signer);
        assert(tx.accounts.payer.is_writable);
    }
}
        "#,
    );
    let payer = account_new();
    vm.account_data.insert(
        payer,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: 10,
        },
    );

    let metas = vec![
        AccountMeta {
            pubkey: Pubkey(vm.stack[0].data),
            is_writable: true,
            is_signer: false,
        },
        AccountMeta {
            pubkey: Pubkey(payer),
            is_writable: true,
            is_signer: true,
        },
    ];

    vm.constructor_expected(0, &metas, &[BorshToken::Address(payer)]);
}
