// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{Bytes, FromVal, IntoVal};

#[test]
fn bls12_381_hash_to_g2_matches_sdk() {
    let runtime = build_solidity(
        r#"contract c {
            function hash(bytes message, bytes dst) public returns (bytes) {
                return bls12_381_hash_to_g2(message, dst);
            }
        }"#,
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;
    let dst = Bytes::from_slice(env, b"BLSSIG-V01-CS01-with-BLS12381G2_XMD:SHA-256_SSWU_RO_");

    for message in [&b"hello soroban"[..], &[]] {
        let message = Bytes::from_slice(env, message);
        let res =
            runtime.invoke_contract(addr, "hash", vec![message.into_val(env), dst.into_val(env)]);
        let expected: Bytes = env
            .crypto()
            .bls12_381()
            .hash_to_g2(&message, &dst)
            .to_bytes()
            .into();
        assert_eq!(Bytes::from_val(env, &res), expected);
        assert_eq!(expected.len(), 192);
    }
}
