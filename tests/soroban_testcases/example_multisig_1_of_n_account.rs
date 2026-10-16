// SPDX-License-Identifier: Apache-2.0

use crate::{authorize_custom_account, SorobanEnv, AUTH_TOKEN};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use soroban_sdk::{
    auth::Context,
    contracttype,
    testutils::{Address as _, BytesN as _},
    vec, Address, BytesN, Env, Error, IntoVal, Symbol, Val,
};

const CONTRACT: &str = include_str!("../../examples/soroban/multisig_1_of_n_account.sol");

#[contracttype]
struct Signature {
    public_key: BytesN<32>,
    signature: BytesN<64>,
}

fn public_key(env: &Env, key: &SigningKey) -> BytesN<32> {
    BytesN::from_array(env, &key.verifying_key().to_bytes())
}

fn sign(env: &Env, key: &SigningKey, payload: &BytesN<32>) -> Val {
    Signature {
        public_key: public_key(env, key),
        signature: BytesN::from_array(env, &key.sign(&payload.to_array()).to_bytes()),
    }
    .into_val(env)
}

fn check_auth(
    env: &Env,
    addr: &Address,
    key: &SigningKey,
    payload: &BytesN<32>,
) -> Result<(), Error> {
    let signature = Signature {
        public_key: public_key(env, key),
        signature: BytesN::from_array(env, &key.sign(&payload.to_array()).to_bytes()),
    };
    let no_contexts: soroban_sdk::Vec<Context> = vec![env];
    env.try_invoke_contract_check_auth::<Error>(
        addr,
        payload,
        signature.into_val(env),
        &no_contexts,
    )
    .map_err(|e| e.unwrap())
}

#[test]
fn example_multisig_1_of_n_account_check_auth() {
    let signer1 = SigningKey::generate(&mut OsRng);
    let signer2 = SigningKey::generate(&mut OsRng);
    let stranger = SigningKey::generate(&mut OsRng);

    let mut runtime = SorobanEnv::new();
    let signers = vec![
        &runtime.env,
        public_key(&runtime.env, &signer1),
        public_key(&runtime.env, &signer2),
    ];
    let addr = runtime.deploy_contract_with_args(CONTRACT, (signers,));
    let env = &runtime.env;
    let payload = BytesN::<32>::random(env);

    // Either configured signer can authorize on its own.
    assert_eq!(check_auth(env, &addr, &signer1, &payload), Ok(()));
    assert_eq!(check_auth(env, &addr, &signer2, &payload), Ok(()));

    // A key that is not a signer gets the `UnknownSigner` contract error.
    assert_eq!(
        check_auth(env, &addr, &stranger, &payload),
        Err(Error::from_contract_error(1))
    );

    // A known signer with a signature over a different payload is rejected.
    let other_payload = BytesN::<32>::random(env);
    let bad = Signature {
        public_key: public_key(env, &signer1),
        signature: BytesN::from_array(env, &signer1.sign(&other_payload.to_array()).to_bytes()),
    };
    assert!(env
        .try_invoke_contract_check_auth::<Error>(&addr, &payload, bad.into_val(env), &vec![env])
        .is_err());
}

#[test]
fn example_multisig_1_of_n_account_authorizes_token_transfer() {
    let signer1 = SigningKey::generate(&mut OsRng);
    let signer2 = SigningKey::generate(&mut OsRng);

    let mut runtime = SorobanEnv::new();
    let signers = vec![
        &runtime.env,
        public_key(&runtime.env, &signer1),
        public_key(&runtime.env, &signer2),
    ];
    let account = runtime.deploy_contract_with_args(CONTRACT, (signers,));
    let token = runtime.deploy_contract(AUTH_TOKEN);
    let env = &runtime.env;
    let args = vec![
        env,
        account.into_val(env),
        Address::generate(env).into_val(env),
        100_i128.into_val(env),
    ];

    let transfer = |signer: &SigningKey| {
        authorize_custom_account(env, &account, &token, "transfer", args.clone(), |payload| {
            sign(env, signer, payload)
        });
        env.try_invoke_contract::<(), Error>(&token, &Symbol::new(env, "transfer"), args.clone())
    };

    // Any one signer is enough; a stranger is not.
    assert!(matches!(transfer(&signer1), Ok(Ok(()))));
    assert!(matches!(transfer(&signer2), Ok(Ok(()))));
    assert!(transfer(&SigningKey::generate(&mut OsRng)).is_err());
}
