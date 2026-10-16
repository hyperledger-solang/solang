// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use soroban_sdk::{vec, Bytes, BytesN, Env, IntoVal, Symbol, Val};

const CONTRACT: &str = r#"
    contract verifier {
        function verify(bytes32 key, bytes32 message, bytes signature) public returns (bool) {
            ed25519_verify(key, message, signature);
            return true;
        }
    }
"#;

/// Returns the encoded (public key, message, signature) arguments for `verify`.
fn signed_args(env: &Env, key: &SigningKey, message: [u8; 32]) -> (Val, Val, Val) {
    let signature = key.sign(&message).to_bytes();
    (
        BytesN::from_array(env, &key.verifying_key().to_bytes()).into_val(env),
        BytesN::from_array(env, &message).into_val(env),
        Bytes::from_slice(env, &signature).into_val(env),
    )
}

#[test]
fn ed25519_verify_valid_signature() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let key = SigningKey::generate(&mut OsRng);
    let (pk, msg, sig) = signed_args(env, &key, [7u8; 32]);

    let res = runtime.invoke_contract(addr, "verify", std::vec![pk, msg, sig]);
    assert!(res.shallow_eq(&true.into_val(env)));
}

#[test]
fn ed25519_verify_rejects_bad_signatures() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let key = SigningKey::generate(&mut OsRng);
    let other = SigningKey::generate(&mut OsRng);
    let (pk, msg, sig) = signed_args(env, &key, [7u8; 32]);
    let (other_pk, other_msg, _) = signed_args(env, &other, [8u8; 32]);
    let short_sig: Val = Bytes::from_slice(env, &[1u8; 10]).into_val(env);

    for (pk, msg, sig) in [
        // signature for a different message
        (pk, other_msg, sig),
        // signature checked against the wrong public key
        (other_pk, msg, sig),
        // signature that is not 64 bytes long
        (pk, msg, short_sig),
    ] {
        let res = env.try_invoke_contract::<Val, Val>(
            addr,
            &Symbol::new(env, "verify"),
            vec![env, pk, msg, sig],
        );
        assert!(res.is_err(), "{res:?}");
    }
}
