// SPDX-License-Identifier: Apache-2.0

use crate::borsh_encoding::BorshToken;
use crate::{account_new, build_solidity, create_program_address, AccountState};

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
    let pda = create_program_address(&vm.stack[0].id, &[b"sunflower"]);

    vm.account_data.insert(
        payer,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: 10,
        },
    );
    vm.account_data.insert(
        pda.0,
        AccountState {
            data: [0; 4096].to_vec(),
            owner: Some(vm.stack[0].id),
            lamports: 0,
        },
    );

    vm.function("new")
        .arguments(&[BorshToken::Address(payer)])
        .accounts(vec![
            ("dataAccount", pda.0),
            ("payer", payer),
            ("systemProgram", [0; 32]),
        ])
        .call();
}
