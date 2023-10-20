// SPDX-License-Identifier: Apache-2.0

use crate::{account_new, build_solidity, AccountMeta, AccountState, BorshToken, Pubkey};
use anchor_syn::idl::types::IdlInstruction;
use num_bigint::BigInt;

#[test]
fn get_balance() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function test(address addr) public view returns (uint64) {
                return addr.balance;
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: 102,
        },
    );

    let returns = vm
        .function("test")
        .arguments(&[BorshToken::Address(new)])
        .remaining_accounts(&[
            AccountMeta {
                pubkey: Pubkey(data_account),
                is_signer: false,
                is_writable: false,
            },
            AccountMeta {
                pubkey: Pubkey(new),
                is_signer: false,
                is_writable: false,
            },
        ])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 64,
            value: BigInt::from(102u8),
        }
    );
}

#[test]
fn send_fails() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function send(address payable addr, uint64 amount) public returns (bool) {
                return addr.send(amount);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: 0,
        },
    );

    let returns = vm
        .function("send")
        .arguments(&[
            BorshToken::Address(new),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(102u8),
            },
        ])
        .remaining_accounts(&[
            AccountMeta {
                pubkey: Pubkey(data_account),
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: Pubkey(new),
                is_signer: false,
                is_writable: true,
            },
        ])
        .call()
        .unwrap();

    assert_eq!(returns, BorshToken::Bool(false));
}

#[test]
fn send_succeeds() {
    let mut vm = build_solidity(
        r#"
        contract c {
            constructor() payable {}

            function send(address payable addr, uint64 amount) public returns (bool) {
                return addr.send(amount);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();

    vm.account_data.get_mut(&data_account).unwrap().lamports = 103;

    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let new = account_new();
    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: 5,
        },
    );

    let returns = vm
        .function("send")
        .arguments(&[
            BorshToken::FixedBytes(new.to_vec()),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(102u8),
            },
        ])
        .remaining_accounts(&[
            AccountMeta {
                pubkey: Pubkey(data_account),
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: Pubkey(new),
                is_signer: false,
                is_writable: true,
            },
        ])
        .call()
        .unwrap();

    assert_eq!(returns, BorshToken::Bool(true));

    assert_eq!(vm.account_data.get_mut(&new).unwrap().lamports, 107);

    assert_eq!(vm.account_data.get_mut(&data_account).unwrap().lamports, 1);
}

#[test]
fn send_overflows() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function send(address payable addr, uint64 amount) public returns (bool) {
                return addr.send(amount);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();

    vm.account_data.get_mut(&data_account).unwrap().lamports = 103;

    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: u64::MAX - 101,
        },
    );

    let returns = vm
        .function("send")
        .arguments(&[
            BorshToken::Address(new),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(102u8),
            },
        ])
        .remaining_accounts(&[
            AccountMeta {
                pubkey: Pubkey(data_account),
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: Pubkey(new),
                is_signer: false,
                is_writable: true,
            },
        ])
        .call()
        .unwrap();

    assert_eq!(returns, BorshToken::Bool(false));

    assert_eq!(
        vm.account_data.get_mut(&new).unwrap().lamports,
        u64::MAX - 101
    );

    assert_eq!(
        vm.account_data.get_mut(&data_account).unwrap().lamports,
        103
    );
}

#[test]
fn transfer_succeeds() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function transfer(address payable addr, uint64 amount) public {
                addr.transfer(amount);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.account_data.get_mut(&data_account).unwrap().lamports = 103;

    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: 5,
        },
    );

    vm.function("transfer")
        .arguments(&[
            BorshToken::Address(new),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(102u8),
            },
        ])
        .remaining_accounts(&[
            AccountMeta {
                pubkey: Pubkey(data_account),
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: Pubkey(new),
                is_signer: false,
                is_writable: true,
            },
        ])
        .call();

    assert_eq!(vm.account_data.get_mut(&new).unwrap().lamports, 107);

    assert_eq!(vm.account_data.get_mut(&data_account).unwrap().lamports, 1);
}

#[test]
fn transfer_fails_not_enough() {
    let mut vm = build_solidity(
        r#"
        contract c {
            function transfer(address payable addr, uint64 amount) public {
                addr.transfer(amount);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.account_data.get_mut(&data_account).unwrap().lamports = 103;

    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: 5,
        },
    );

    let res = vm
        .function("transfer")
        .arguments(&[
            BorshToken::Address(new),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(104u8),
            },
        ])
        .remaining_accounts(&[
            AccountMeta {
                pubkey: Pubkey(data_account),
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: Pubkey(new),
                is_signer: false,
                is_writable: true,
            },
        ])
        .must_fail();
    assert!(res.is_err());

    // Ensure the balance in the account has not overflowed
    assert_eq!(vm.account_data[&data_account].lamports, 103);
    assert_eq!(vm.account_data[&new].lamports, 5);

    vm.function("transfer")
        .arguments(&[
            BorshToken::Address(new),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(103u8),
            },
        ])
        .remaining_accounts(&[
            AccountMeta {
                pubkey: Pubkey(data_account),
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: Pubkey(new),
                is_signer: false,
                is_writable: true,
            },
        ])
        .call();

    assert_eq!(vm.account_data[&data_account].lamports, 0);
    assert_eq!(vm.account_data[&new].lamports, 108);
}

#[test]
fn transfer_fails_overflow() {
    let mut vm = build_solidity(
        r#"
        contract c {
            constructor() payable {}

            function transfer(address payable addr, uint64 amount) public {
                addr.transfer(amount);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.account_data.get_mut(&data_account).unwrap().lamports = 104;

    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: u64::MAX - 100,
        },
    );

    let res = vm
        .function("transfer")
        .arguments(&[
            BorshToken::FixedBytes(new.to_vec()),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(104u8),
            },
        ])
        .remaining_accounts(&[
            AccountMeta {
                pubkey: Pubkey(data_account),
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: Pubkey(new),
                is_writable: false,
                is_signer: true,
            },
        ])
        .must_fail();
    assert!(res.is_err());

    // Ensure no change in the values
    assert_eq!(vm.account_data[&new].lamports, u64::MAX - 100);
    assert_eq!(vm.account_data[&data_account].lamports, 104);

    vm.function("transfer")
        .arguments(&[
            BorshToken::FixedBytes(new.to_vec()),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(100u8),
            },
        ])
        .remaining_accounts(&[
            AccountMeta {
                pubkey: Pubkey(data_account),
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: Pubkey(new),
                is_writable: false,
                is_signer: true,
            },
        ])
        .call();

    assert_eq!(vm.account_data[&new].lamports, u64::MAX);
    assert_eq!(vm.account_data[&data_account].lamports, 4);
}

#[test]
fn fallback() {
    let mut vm = build_solidity(
        r#"
        contract c {
            fallback() external {
                print("fallback");
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    if let Some(idl) = &vm.stack[0].idl {
        let mut idl = idl.clone();

        idl.instructions.push(IdlInstruction {
            name: "extinct".to_string(),
            docs: None,
            accounts: vec![],
            args: vec![],
            returns: None,
        });

        vm.stack[0].idl = Some(idl);
    }

    vm.function("extinct").call();

    assert_eq!(vm.logs, "fallback");
}

#[test]
fn value_overflows() {
    let mut vm = build_solidity(
        r#"
        contract c {
            constructor() payable {}

            function send(address payable addr, uint128 amount) public returns (bool) {
                return addr.send(amount);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.account_data.get_mut(&data_account).unwrap().lamports = 103;

    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let new = account_new();

    vm.account_data.insert(
        new,
        AccountState {
            data: Vec::new(),
            owner: None,
            lamports: u64::MAX - 101,
        },
    );

    let res = vm
        .function("send")
        .arguments(&[
            BorshToken::Address(new),
            BorshToken::Uint {
                width: 128,
                value: BigInt::from(u64::MAX as u128 + 1),
            },
        ])
        .remaining_accounts(&[
            AccountMeta {
                pubkey: Pubkey(data_account),
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: Pubkey(new),
                is_signer: false,
                is_writable: true,
            },
        ])
        .must_fail();
    assert_eq!(res.unwrap(), 4294967296);

    let res = vm
        .function("send")
        .arguments(&[
            BorshToken::Address(new),
            BorshToken::Uint {
                width: 128,
                value: BigInt::from(u128::MAX),
            },
        ])
        .remaining_accounts(&[
            AccountMeta {
                pubkey: Pubkey(data_account),
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: Pubkey(new),
                is_signer: false,
                is_writable: true,
            },
        ])
        .must_fail();

    assert_eq!(res.unwrap(), 4294967296);

    let returns = vm
        .function("send")
        .arguments(&[
            BorshToken::Address(new),
            BorshToken::Uint {
                width: 128,
                value: BigInt::from(102u8),
            },
        ])
        .remaining_accounts(&[
            AccountMeta {
                pubkey: Pubkey(data_account),
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: Pubkey(new),
                is_signer: false,
                is_writable: true,
            },
        ])
        .call()
        .unwrap();

    assert_eq!(returns, BorshToken::Bool(false));

    assert_eq!(
        vm.account_data.get_mut(&new).unwrap().lamports,
        u64::MAX - 101
    );

    assert_eq!(
        vm.account_data.get_mut(&data_account).unwrap().lamports,
        103
    );
}
