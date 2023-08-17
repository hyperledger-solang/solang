// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, Account, AccountState, BorshToken};
use base58::FromBase58;
use ed25519_dalek::{Signature, Signer, SigningKey};
use rand::rngs::OsRng;
use serde_derive::Serialize;
use std::convert::TryInto;
use std::mem::size_of;

#[derive(Serialize)]
#[repr(C)]
struct Instructions {
    num_instructions: u16,
    instruction_offset: u16,
    num_accounts: u16,
    /* assume no accounts */
    program_id: [u8; 32],
    instruction_len: u16,
}

#[derive(Serialize)]
#[repr(C)]
pub struct Ed25519SignatureOffsets {
    num_signatures: u8,
    padding: u8,
    signature_offset: u16,            // offset to ed25519 signature of 64 bytes
    signature_instruction_index: u16, // instruction index to find signature
    public_key_offset: u16,           // offset to public key of 32 bytes
    public_key_instruction_index: u16, // instruction index to find public key
    message_data_offset: u16,         // offset to start of message data
    message_data_size: u16,           // size of message data
    message_instruction_index: u16,   // index of instruction data to get message data
}

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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let instructions_account: Account = "Sysvar1nstructions1111111111111111111111111"
        .from_base58()
        .unwrap()
        .try_into()
        .unwrap();
    vm.account_data
        .insert(instructions_account, AccountState::default());

    let mut csprng = OsRng;
    let keypair: SigningKey = SigningKey::generate(&mut csprng);

    let message: &[u8] =
        b"This is a test of the ed25519 sig check for the ed25519 signature check program";

    let signature: Signature = keypair.sign(message);

    let signature_bs = signature.to_bytes().to_vec();

    println!(
        "T: PUB: {}",
        hex::encode(keypair.verifying_key().to_bytes())
    );
    println!("T: SIG: {}", hex::encode(&signature_bs));
    println!("T: MES: {}", hex::encode(message));

    let returns = vm
        .function("verify")
        .arguments(&[
            BorshToken::Address(keypair.verifying_key().to_bytes()),
            BorshToken::Bytes(message.to_vec()),
            BorshToken::Bytes(signature_bs.clone()),
        ])
        .accounts(vec![("SysvarInstruction", instructions_account)])
        .call()
        .unwrap();

    assert_eq!(returns, BorshToken::Bool(false));

    let instructions_account: Account = "Sysvar1nstructions1111111111111111111111111"
        .from_base58()
        .unwrap()
        .try_into()
        .unwrap();
    let instructions =
        encode_instructions(&keypair.verifying_key().to_bytes(), &signature_bs, message);

    vm.account_data.insert(
        instructions_account,
        AccountState {
            data: instructions,
            owner: None,
            lamports: 0,
        },
    );

    println!("Now try for real");

    let returns = vm
        .function("verify")
        .arguments(&[
            BorshToken::Address(keypair.verifying_key().to_bytes()),
            BorshToken::Bytes(message.to_vec()),
            BorshToken::Bytes(signature_bs.clone()),
        ])
        .accounts(vec![("SysvarInstruction", instructions_account)])
        .call()
        .unwrap();

    assert_eq!(returns, BorshToken::Bool(true));

    println!("now try with bad signature");

    // flip a bit and make sure it no longer verifies
    let mut signature_copy = signature_bs.clone();
    signature_copy[2] ^= 0x80;

    let instructions = encode_instructions(
        &keypair.verifying_key().to_bytes(),
        &signature_copy,
        message,
    );

    vm.account_data.insert(
        instructions_account,
        AccountState {
            data: instructions,
            owner: None,
            lamports: 0,
        },
    );

    let returns = vm
        .function("verify")
        .arguments(&[
            BorshToken::Address(keypair.verifying_key().to_bytes()),
            BorshToken::Bytes(message.to_vec()),
            BorshToken::Bytes(signature_bs),
        ])
        .accounts(vec![("SysvarInstruction", instructions_account)])
        .call()
        .unwrap();

    assert_eq!(returns, BorshToken::Bool(false));
}

fn encode_instructions(public_key: &[u8], signature: &[u8], message: &[u8]) -> Vec<u8> {
    let offsets = Ed25519SignatureOffsets {
        num_signatures: 1,
        padding: 0,
        signature_offset: size_of::<Ed25519SignatureOffsets>() as u16,
        signature_instruction_index: 0,
        public_key_offset: (size_of::<Ed25519SignatureOffsets>() + signature.len()) as u16,
        public_key_instruction_index: 0,
        message_data_offset: (size_of::<Ed25519SignatureOffsets>()
            + signature.len()
            + public_key.len()) as u16,
        message_instruction_index: 0,
        message_data_size: message.len() as u16,
    };

    let mut ed25519_instruction = bincode::serialize(&offsets).unwrap();
    ed25519_instruction.extend_from_slice(signature);
    ed25519_instruction.extend_from_slice(public_key);
    ed25519_instruction.extend_from_slice(message);

    let instr = Instructions {
        num_instructions: 1,
        instruction_offset: 4,
        num_accounts: 0,
        program_id: "Ed25519SigVerify111111111111111111111111111"
            .from_base58()
            .unwrap()
            .try_into()
            .unwrap(),
        instruction_len: ed25519_instruction.len() as u16,
    };

    let mut instructions = bincode::serialize(&instr).unwrap();

    instructions.extend_from_slice(&ed25519_instruction);

    instructions
}
