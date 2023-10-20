// SPDX-License-Identifier: Apache-2.0

use crate::{account_new, build_solidity, build_solidity_with_cache, AccountState, BorshToken};
use borsh::to_vec;
use borsh_derive::BorshSerialize;
use num_bigint::BigInt;
use solang::file_resolver::FileResolver;

#[test]
fn use_authority() {
    let mut vm = build_solidity(include_str!("../../docs/examples/solana/use_authority.sol"));

    let authority = account_new();
    let another_authority = account_new();

    vm.account_data.insert(
        authority,
        AccountState {
            data: vec![],
            owner: Some([0u8; 32]),
            lamports: 0,
        },
    );

    vm.account_data.insert(
        another_authority,
        AccountState {
            data: vec![],
            owner: Some([0; 32]),
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
        .accounts(vec![
            ("dataAccount", data_account),
            ("authorityAccount", another_authority),
        ])
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
        .accounts(vec![
            ("dataAccount", data_account),
            ("authorityAccount", authority),
        ])
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

#[test]
fn token_account() {
    let mut cache = FileResolver::default();
    cache.set_file_contents(
        "spl_token.sol",
        include_str!("../../solana-library/spl_token.sol").to_string(),
    );
    let src = r#"
    import './spl_token.sol';

contract Foo {
    @account(add)
    function token_account() external returns (SplToken.TokenAccountData) {
        return SplToken.get_token_account_data(tx.accounts.add);
    }
}
    "#;
    cache.set_file_contents("test.sol", src.to_string());

    let mut vm = build_solidity_with_cache(cache);

    #[derive(BorshSerialize)]
    struct TokenAccount {
        mint_account: [u8; 32],
        owner: [u8; 32],
        balance: u64,
        delegate_present: u32,
        delegate: [u8; 32],
        state: u8,
        is_native_present: u32,
        is_native: u64,
        delegated_amount: u64,
        close_authority_present: u32,
        close_authority: [u8; 32],
    }

    let mut data = TokenAccount {
        mint_account: account_new(),
        owner: account_new(),
        balance: 234,
        delegate_present: 0,
        delegate: account_new(),
        state: 1,
        is_native_present: 0,
        is_native: 234,
        delegated_amount: 1346,
        close_authority_present: 0,
        close_authority: account_new(),
    };

    let encoded = to_vec(&data).unwrap();

    let account = account_new();
    vm.account_data.insert(
        account,
        AccountState {
            owner: None,
            lamports: 0,
            data: encoded,
        },
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let res = vm
        .function("token_account")
        .accounts(vec![("add", account)])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        res,
        vec![
            BorshToken::Address(data.mint_account),
            BorshToken::Address(data.owner),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(data.balance)
            },
            BorshToken::Bool(data.delegate_present > 0),
            BorshToken::Address(data.delegate),
            BorshToken::Uint {
                width: 8,
                value: BigInt::from(data.state)
            },
            BorshToken::Bool(data.is_native_present > 0),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(data.is_native)
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(data.delegated_amount)
            },
            BorshToken::Bool(data.close_authority_present > 0),
            BorshToken::Address(data.close_authority)
        ]
    );

    data.delegate_present = 1;
    data.is_native_present = 1;
    data.close_authority_present = 1;

    let encoded = to_vec(&data).unwrap();
    vm.account_data.get_mut(&account).unwrap().data = encoded;

    let res = vm
        .function("token_account")
        .accounts(vec![("add", account)])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        res,
        vec![
            BorshToken::Address(data.mint_account),
            BorshToken::Address(data.owner),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(data.balance)
            },
            BorshToken::Bool(data.delegate_present > 0),
            BorshToken::Address(data.delegate),
            BorshToken::Uint {
                width: 8,
                value: BigInt::from(data.state)
            },
            BorshToken::Bool(data.is_native_present > 0),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(data.is_native)
            },
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(data.delegated_amount)
            },
            BorshToken::Bool(data.close_authority_present > 0),
            BorshToken::Address(data.close_authority)
        ]
    );
}

#[test]
fn mint_account() {
    let mut cache = FileResolver::default();
    cache.set_file_contents(
        "spl_token.sol",
        include_str!("../../solana-library/spl_token.sol").to_string(),
    );
    let src = r#"
    import './spl_token.sol';

contract Foo {
    @account(add)
    function mint_account() external returns (SplToken.MintAccountData) {
        return SplToken.get_mint_account_data(tx.accounts.add);
    }
}
    "#;
    cache.set_file_contents("test.sol", src.to_string());

    let mut vm = build_solidity_with_cache(cache);

    #[derive(BorshSerialize)]
    struct MintAccountData {
        authority_present: u32,
        mint_authority: [u8; 32],
        supply: u64,
        decimals: u8,
        is_initialized: bool,
        freeze_authority_present: u32,
        freeze_authority: [u8; 32],
    }

    let mut data = MintAccountData {
        authority_present: 0,
        mint_authority: account_new(),
        supply: 450,
        decimals: 4,
        is_initialized: false,
        freeze_authority_present: 0,
        freeze_authority: account_new(),
    };

    let encoded = to_vec(&data).unwrap();
    let account = account_new();
    vm.account_data.insert(
        account,
        AccountState {
            owner: None,
            lamports: 0,
            data: encoded,
        },
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let res = vm
        .function("mint_account")
        .accounts(vec![("add", account)])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        res,
        vec![
            BorshToken::Bool(data.authority_present > 0),
            BorshToken::Address(data.mint_authority),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(data.supply)
            },
            BorshToken::Uint {
                width: 8,
                value: BigInt::from(data.decimals)
            },
            BorshToken::Bool(data.is_initialized),
            BorshToken::Bool(data.freeze_authority_present > 0),
            BorshToken::Address(data.freeze_authority)
        ]
    );

    data.authority_present = 1;
    data.is_initialized = true;
    data.freeze_authority_present = 1;
    let encoded = to_vec(&data).unwrap();
    vm.account_data.get_mut(&account).unwrap().data = encoded;

    let res = vm
        .function("mint_account")
        .accounts(vec![("add", account)])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        res,
        vec![
            BorshToken::Bool(data.authority_present > 0),
            BorshToken::Address(data.mint_authority),
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(data.supply)
            },
            BorshToken::Uint {
                width: 8,
                value: BigInt::from(data.decimals)
            },
            BorshToken::Bool(data.is_initialized),
            BorshToken::Bool(data.freeze_authority_present > 0),
            BorshToken::Address(data.freeze_authority)
        ]
    );
}
