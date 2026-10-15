// SPDX-License-Identifier: Apache-2.0

use crate::SorobanEnv;
use soroban_sdk::{
    crypto::bls12_381::{Fr, G1Affine},
    testutils::BytesN as _,
    vec, Address, Bytes, BytesN, Env, Error, IntoVal, Val, U256,
};

const CONTRACT: &str = include_str!("../../examples/soroban/bls_signature.sol");
const DST: &[u8] = b"BLSSIG-V01-CS01-with-BLS12381G2_XMD:SHA-256_SSWU_RO_";

// Two of the key pairs from the upstream example's test.
const SECRET_KEYS: [&str; 2] = [
    "18a5ac3cfa6d0b10437a92c96f553311fc0e25500d691ae4b26581e6f925ec83",
    "738dbecafa122ee3c953f07e78461a4281cadec00c869098bac48c8c57b63374",
];
const PUBLIC_KEYS: [&str; 2] = [
    "0914e32703bad05ccf4180e240e44e867b26580f36e09331997b2e9effe1f509b1a804fc7ba1f1334c8d41f060dd72550901c5549caef45212a236e288a785d762a087092c769bfa79611b96d73521ddd086b7e05b5c7e4210f50c2ee832e183",
    "05f4708a013699229f67d0e16f7c2af8a6557d6d11b737286cfb9429e092c31c412f623d61c7de259c33701aa5387b5004e2c03e8b7ea2740b10a5b4fd050eecca45ccf5588d024cbb7adc963006c29d45a38cb7a06ce2ac45fce52fc0d36572",
];

fn from_hex<const N: usize>(env: &Env, hex: &str) -> BytesN<N> {
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    BytesN::from_array(env, &bytes.try_into().unwrap())
}

/// The sum of the signers' public keys.
fn aggregate_pk(env: &Env) -> Bytes {
    let bls = env.crypto().bls12_381();
    let pks = PUBLIC_KEYS.map(|pk| G1Affine::from_bytes(from_hex(env, pk)));
    bls.g1_add(&pks[0], &pks[1]).to_bytes().into()
}

/// Every signer signs `payload`; the signatures are summed into one.
fn aggregate_sig(env: &Env, payload: &BytesN<32>) -> Val {
    let bls = env.crypto().bls12_381();
    let msg_g2 = bls.hash_to_g2(&payload.clone().into(), &Bytes::from_slice(env, DST));
    let sks =
        SECRET_KEYS.map(|sk| Fr::from(U256::from_be_bytes(env, &from_hex::<32>(env, sk).into())));
    let sig = bls.g2_msm(
        vec![env, msg_g2.clone(), msg_g2],
        vec![env, sks[0].clone(), sks[1].clone()],
    );
    Bytes::from(sig.to_bytes()).into_val(env)
}

fn check_auth(env: &Env, addr: &Address, payload: &BytesN<32>, sig: Val) -> Result<(), Error> {
    env.try_invoke_contract_check_auth::<Error>(addr, payload, sig, &vec![env])
        .map_err(|e| e.unwrap())
}

#[test]
fn example_bls_signature_check_auth() {
    let mut runtime = SorobanEnv::new();
    let agg_pk = aggregate_pk(&runtime.env);
    let addr = runtime.deploy_contract_with_args(CONTRACT, (agg_pk,));
    let env = &runtime.env;
    let payload = BytesN::<32>::random(env);

    // The aggregated signature of all signers is accepted.
    assert_eq!(
        check_auth(env, &addr, &payload, aggregate_sig(env, &payload)),
        Ok(())
    );

    // A valid signature, but over a different payload, gets `InvalidSignature`.
    let other_payload = BytesN::<32>::random(env);
    assert_eq!(
        check_auth(env, &addr, &payload, aggregate_sig(env, &other_payload)),
        Err(Error::from_contract_error(1))
    );
}

#[test]
fn example_bls_signature_increment_requires_own_auth() {
    let mut runtime = SorobanEnv::new();
    let agg_pk = aggregate_pk(&runtime.env);
    let addr = runtime.deploy_contract_with_args(CONTRACT, (agg_pk,));
    let env = &runtime.env;

    // Without any authorization, `increment` fails.
    assert!(env
        .try_invoke_contract::<u32, Error>(
            &addr,
            &soroban_sdk::Symbol::new(env, "increment"),
            vec![env]
        )
        .is_err());

    env.mock_all_auths();
    let count: u32 = env.invoke_contract(
        &addr,
        &soroban_sdk::Symbol::new(env, "increment"),
        vec![env],
    );
    assert_eq!(count, 1);
    // The account contract itself is the address that had to authorize.
    assert_eq!(env.auths()[0].0, addr);
}
