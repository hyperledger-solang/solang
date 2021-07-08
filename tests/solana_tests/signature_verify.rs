use crate::build_solidity;
use ed25519_dalek::{Keypair, Signature, Signer};
use ethabi::Token;

#[test]
fn verify() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function verify(address addr, bytes message, bytes signature) public returns (bool) {
                return signatureVerify(addr, message, signature);
            }
        }"#,
    );

    vm.constructor("foo", &[]);

    let mut csprng = rand::thread_rng();
    let keypair: Keypair = Keypair::generate(&mut csprng);

    let message: &[u8] =
        b"This is a test of the ed25519 sig check for the sol_ed25519_sig_check syscall";

    let signature: Signature = keypair.sign(message);

    let mut signature_bs = signature.to_bytes().to_vec();

    let returns = vm.function(
        "verify",
        &[
            Token::FixedBytes(keypair.public.to_bytes().to_vec()),
            Token::Bytes(message.to_vec()),
            Token::Bytes(signature_bs.clone()),
        ],
        &[],
    );

    assert_eq!(returns, vec![Token::Bool(true)]);

    // flip a bit and make sure it no longer verifies
    signature_bs[2] ^= 0x80;

    let returns = vm.function(
        "verify",
        &[
            Token::FixedBytes(keypair.public.to_bytes().to_vec()),
            Token::Bytes(message.to_vec()),
            Token::Bytes(signature_bs),
        ],
        &[],
    );

    assert_eq!(returns, vec![Token::Bool(false)]);
}
