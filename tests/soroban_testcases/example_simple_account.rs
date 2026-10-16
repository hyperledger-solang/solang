// SPDX-License-Identifier: Apache-2.0

use crate::{authorize_custom_account, SorobanEnv, AUTH_TOKEN};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use soroban_sdk::{
    auth::Context,
    testutils::{Address as _, BytesN as _},
    vec, Address, BytesN, Env, Error, IntoVal, Symbol, Val,
};

const CONTRACT: &str = include_str!("../../examples/soroban/simple_account.sol");

fn sign(env: &Env, key: &SigningKey, payload: &BytesN<32>) -> Val {
    let signature: BytesN<64> = BytesN::from_array(env, &key.sign(&payload.to_array()).to_bytes());
    signature.into_val(env)
}

fn deploy(runtime: &mut SorobanEnv, key: &SigningKey) -> Address {
    let public_key = BytesN::from_array(&runtime.env, &key.verifying_key().to_bytes());
    runtime.deploy_contract_with_args(CONTRACT, (public_key,))
}

#[test]
fn example_simple_account_check_auth() {
    let key = SigningKey::generate(&mut OsRng);
    let mut runtime = SorobanEnv::new();
    let addr = &deploy(&mut runtime, &key);
    let env = &runtime.env;
    let no_contexts: soroban_sdk::Vec<Context> = vec![env];
    let payload = BytesN::<32>::random(env);

    // `__check_auth` can only be called by the host during `require_auth`; this
    // test utility emulates that call.
    env.try_invoke_contract_check_auth::<Error>(
        addr,
        &payload,
        sign(env, &key, &payload),
        &no_contexts,
    )
    .unwrap();

    // A random signature is rejected.
    assert!(env
        .try_invoke_contract_check_auth::<Error>(
            addr,
            &payload,
            BytesN::<64>::random(env).into(),
            &no_contexts,
        )
        .is_err());

    // A signature from a key that is not the owner is rejected.
    let stranger = SigningKey::generate(&mut OsRng);
    assert!(env
        .try_invoke_contract_check_auth::<Error>(
            addr,
            &payload,
            sign(env, &stranger, &payload),
            &no_contexts,
        )
        .is_err());
}

#[test]
fn example_simple_account_authorizes_token_transfer() {
    let key = SigningKey::generate(&mut OsRng);
    let mut runtime = SorobanEnv::new();
    let account = deploy(&mut runtime, &key);
    let token = runtime.deploy_contract(AUTH_TOKEN);
    let env = &runtime.env;
    let args = vec![
        env,
        account.into_val(env),
        Address::generate(env).into_val(env),
        100_i128.into_val(env),
    ];

    // `token.transfer(account, ...)` calls `account.requireAuth()`, so the host runs the
    // account's `__check_auth` with the signature attached to the transaction.
    let transfer = |signer: &SigningKey| {
        authorize_custom_account(env, &account, &token, "transfer", args.clone(), |payload| {
            sign(env, signer, payload)
        });
        env.try_invoke_contract::<(), Error>(&token, &Symbol::new(env, "transfer"), args.clone())
    };

    assert!(matches!(transfer(&key), Ok(Ok(()))));
    assert!(transfer(&SigningKey::generate(&mut OsRng)).is_err());
}
