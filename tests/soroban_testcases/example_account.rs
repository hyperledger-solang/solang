// SPDX-License-Identifier: Apache-2.0

use crate::{authorize_custom_account, SorobanEnv, AUTH_TOKEN};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use soroban_sdk::{
    auth::{Context, ContractContext},
    contracttype,
    testutils::{Address as _, BytesN as _},
    vec, Address, BytesN, Env, Error, IntoVal, Symbol, Val,
};

const CONTRACT: &str = include_str!("../../examples/soroban/account.sol");

#[contracttype]
#[derive(Clone)]
struct AccSignature {
    public_key: BytesN<32>,
    signature: BytesN<64>,
}

fn public_key(env: &Env, key: &SigningKey) -> BytesN<32> {
    BytesN::from_array(env, &key.verifying_key().to_bytes())
}

/// The signatures of `keys`, in the given order.
fn sign(env: &Env, keys: &[&SigningKey], payload: &BytesN<32>) -> Val {
    let mut signatures = vec![env];
    for key in keys {
        signatures.push_back(AccSignature {
            public_key: public_key(env, key),
            signature: BytesN::from_array(env, &key.sign(&payload.to_array()).to_bytes()),
        });
    }
    signatures.into_val(env)
}

/// Two signers, sorted by public key as the account requires.
fn deploy() -> (SorobanEnv, Address, [SigningKey; 2]) {
    let mut keys = [
        SigningKey::generate(&mut OsRng),
        SigningKey::generate(&mut OsRng),
    ];
    keys.sort_by_key(|key| key.verifying_key().to_bytes());
    let mut runtime = SorobanEnv::new();
    let signers = vec![
        &runtime.env,
        public_key(&runtime.env, &keys[0]),
        public_key(&runtime.env, &keys[1]),
    ];
    let account = runtime.deploy_contract_with_args(CONTRACT, (signers,));
    (runtime, account, keys)
}

#[test]
fn example_account_check_auth_errors() {
    let (runtime, account, [k0, k1]) = deploy();
    let env = &runtime.env;
    let payload = BytesN::<32>::random(env);
    let call_to = |contract: &Address, amount: i128| {
        let ctx = Context::Contract(ContractContext {
            contract: contract.clone(),
            fn_name: Symbol::new(env, "transfer"),
            args: vec![
                env,
                account.into_val(env),
                account.into_val(env),
                amount.into_val(env),
            ],
        });
        vec![env, ctx]
    };
    let check_auth = |keys: &[&SigningKey], contexts| {
        env.try_invoke_contract_check_auth::<Error>(
            &account,
            &payload,
            sign(env, keys, &payload),
            &contexts,
        )
        .map_err(|e| e.unwrap())
    };
    let token = Address::generate(env);

    assert_eq!(check_auth(&[&k0, &k1], call_to(&token, 5)), Ok(()));
    assert_eq!(check_auth(&[&k0], call_to(&token, 5)), Ok(()));
    // NotEnoughSigners: one signer may not authorize calls to the account itself
    assert_eq!(
        check_auth(&[&k0], call_to(&account, 5)),
        Err(Error::from_contract_error(1))
    );
    // BadSignatureOrder
    assert_eq!(
        check_auth(&[&k1, &k0], call_to(&token, 5)),
        Err(Error::from_contract_error(3))
    );
    // UnknownSigner
    let stranger = SigningKey::generate(&mut OsRng);
    assert_eq!(
        check_auth(&[&stranger], call_to(&token, 5)),
        Err(Error::from_contract_error(4))
    );
}

#[test]
fn example_account_token_spend_limit() {
    let (mut runtime, account, [k0, k1]) = deploy();
    let token = runtime.deploy_contract(AUTH_TOKEN);
    let env = &runtime.env;

    // Calls `contract.fn_name(args)`, authorized by the account with the signatures of `keys`.
    let invoke =
        |contract: &Address, fn_name: &str, args: soroban_sdk::Vec<Val>, keys: &[&SigningKey]| {
            authorize_custom_account(env, &account, contract, fn_name, args.clone(), |payload| {
                sign(env, keys, payload)
            });
            env.try_invoke_contract::<(), Error>(contract, &Symbol::new(env, fn_name), args)
        };
    let add_limit = vec![env, token.into_val(env), 1000_i128.into_val(env)];
    let transfer = |amount: i128| {
        vec![
            env,
            account.into_val(env),
            Address::generate(env).into_val(env),
            amount.into_val(env),
        ]
    };

    // Without a limit, a single signer may transfer any amount.
    assert!(matches!(
        invoke(&token, "transfer", transfer(5000), &[&k0]),
        Ok(Ok(()))
    ));

    // Changing the account needs every signer.
    assert!(invoke(&account, "add_limit", add_limit.clone(), &[&k0]).is_err());
    assert!(matches!(
        invoke(&account, "add_limit", add_limit, &[&k0, &k1]),
        Ok(Ok(()))
    ));

    // Now a single signer may transfer up to the limit; every signer may transfer more.
    assert!(matches!(
        invoke(&token, "transfer", transfer(1000), &[&k1]),
        Ok(Ok(()))
    ));
    assert!(invoke(&token, "transfer", transfer(1001), &[&k1]).is_err());
    assert!(matches!(
        invoke(&token, "transfer", transfer(5000), &[&k0, &k1]),
        Ok(Ok(()))
    ));
}
