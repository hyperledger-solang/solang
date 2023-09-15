// SPDX-License-Identifier: Apache-2.0

use crate::borsh_encoding::BorshToken;
use crate::{account_new, build_solidity, create_program_address, AccountState};
use anchor_syn::idl::{IdlAccount, IdlAccountItem, IdlInstruction};

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

#[test]
fn fallback_magic() {
    let mut vm = build_solidity(
        r#"
        @program_id("5afzkvPkrshqu4onwBCsJccb1swrt4JdAjnpzK8N4BzZ")
contract hatchling {
    string name;
    address private origin;

    constructor(string id, address parent) {
        require(id != "", "name must be provided");
        name = id;
        origin = parent;
    }

    function root() public returns (address) {
        return origin;
    }

    fallback() external {
        name = "wrong";
    }
}

        "#,
    );

    let data_account = vm.initialize_data_account();
    let parent = account_new();

    vm.function("new")
        .arguments(&[
            BorshToken::String("my_id".to_string()),
            BorshToken::Address(parent),
        ])
        .accounts(vec![("dataAccount", data_account)])
        .call();

    if let Some(idl) = &mut vm.stack[0].idl {
        idl.instructions.push(IdlInstruction {
            name: "wrong".to_string(),
            docs: None,
            accounts: vec![IdlAccountItem::IdlAccount(IdlAccount {
                name: "dataAccount".to_string(),
                is_signer: false,
                is_optional: None,
                docs: None,
                pda: None,
                is_mut: true,
                relations: vec![],
            })],
            args: vec![],
            returns: None,
        });
    }

    // This should work
    vm.function("wrong")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    vm.account_data.insert(parent, AccountState::default());

    // This should fail
    let res = vm
        .function("wrong")
        .accounts(vec![("dataAccount", parent)])
        .must_fail();

    assert_eq!(res.unwrap(), 2);
}
