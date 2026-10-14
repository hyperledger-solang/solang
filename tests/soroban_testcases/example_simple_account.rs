// SPDX-License-Identifier: Apache-2.0

use crate::SorobanEnv;
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use soroban_sdk::{auth::Context, testutils::BytesN as _, vec, BytesN, Env, Error, IntoVal, Val};

const CONTRACT: &str = r#"
contract simple_account {
    bytes32 instance owner;

    constructor(bytes32 public_key) {
        owner = public_key;
    }

    function __check_auth(
        bytes32 signature_payload,
        bytes signature,
        bytes[] auth_contexts
    ) public view {
        ed25519_verify(owner, signature_payload, signature);
    }
}
"#;

fn sign(env: &Env, key: &SigningKey, payload: &BytesN<32>) -> Val {
    let signature: BytesN<64> = BytesN::from_array(env, &key.sign(&payload.to_array()).to_bytes());
    signature.into_val(env)
}

#[test]
fn example_simple_account_check_auth() {
    let key = SigningKey::generate(&mut OsRng);
    let mut runtime = SorobanEnv::new();
    let public_key = BytesN::from_array(&runtime.env, &key.verifying_key().to_bytes());
    let addr = &runtime.deploy_contract_with_args(CONTRACT, (public_key,));
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
