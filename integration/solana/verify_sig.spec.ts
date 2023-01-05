// SPDX-License-Identifier: Apache-2.0

import { Keypair, Ed25519Program, SYSVAR_INSTRUCTIONS_PUBKEY, PublicKey } from '@solana/web3.js';
import expect from 'expect';
import nacl from 'tweetnacl';
import { loadContract } from './setup';
import { Program } from '@project-serum/anchor';

describe('Signature Check', function () {
    this.timeout(150000);

    let program: Program;
    let storage: Keypair;
    let payer: Keypair;

    before(async function () {
        ({ program, storage, payer } = await loadContract('verify_sig'));
    });

    it('check valid signature', async function () {
        const message = Buffer.from('Foobar');
        const signature = nacl.sign.detached(message, payer.secretKey);

        let instr1 = Ed25519Program.createInstructionWithPublicKey({
            publicKey: payer.publicKey.toBytes(),
            message,
            signature,
            instructionIndex: 0
        });

        const result = await program.methods.verify(payer.publicKey, message, Buffer.from(signature))
            .preInstructions([instr1])
            .accounts({ dataAccount: storage.publicKey })
            .remainingAccounts([{ pubkey: SYSVAR_INSTRUCTIONS_PUBKEY, isSigner: false, isWritable: false }])
            .view();

        expect(result).toEqual(true);
    });

    it('check invalid signature', async function () {
        const message = Buffer.from('Foobar');
        const signature = nacl.sign.detached(message, payer.secretKey);

        const broken_signature = Buffer.from(signature);

        broken_signature[1] ^= 1;

        let instr1 = Ed25519Program.createInstructionWithPublicKey({
            publicKey: payer.publicKey.toBytes(),
            message,
            signature,
            instructionIndex: 0
        });

        const result = await program.methods.verify(payer.publicKey, message, Buffer.from(broken_signature))
            .preInstructions([instr1])
            .accounts({ dataAccount: storage.publicKey })
            .remainingAccounts([{ pubkey: SYSVAR_INSTRUCTIONS_PUBKEY, isSigner: false, isWritable: false }])
            .view();

        expect(result).toEqual(false);
    });
});
