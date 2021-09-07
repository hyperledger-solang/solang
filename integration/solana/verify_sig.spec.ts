import expect from 'expect';
import { establishConnection } from './index';
import nacl from 'tweetnacl';
import {
    Ed25519Program, SYSVAR_INSTRUCTIONS_PUBKEY
} from '@solana/web3.js';

describe('Deploy solang contract and test', () => {
    it('verify_signature', async function () {
        this.timeout(50000);

        // This test depends on: https://github.com/solana-labs/solana/pull/19685
        //this.skip();

        let conn = await establishConnection();

        let prog = await conn.loadProgram("bundle.so", "verify_sig.abi");

        let message = Buffer.from('In the temple of love you hide together');

        let signature = nacl.sign.detached(message, prog.contractStorageAccount.secretKey);

        let instr = Ed25519Program.createInstructionWithPublicKey({
            publicKey: prog.contractStorageAccount.publicKey.toBuffer(),
            message,
            signature,
        });

        // call the constructor
        await prog.call_constructor(conn, 'verify_sig', []);

        let res = await prog.call_function(
            conn,
            "verify",
            ['0x' + prog.contractStorageAccount.publicKey.toBuffer().toString('hex'), '0x' + message.toString('hex'), '0x' + Buffer.from(signature).toString('hex')],
            [SYSVAR_INSTRUCTIONS_PUBKEY],
            [],
            [],
            [instr]
        );

        expect(res["0"]).toBe(true);

        signature[2] ^= 0x40;

        res = await prog.call_function(conn,
            "verify",
            ['0x' + prog.contractStorageAccount.publicKey.toBuffer().toString('hex'), '0x' + message.toString('hex'), '0x' + Buffer.from(signature).toString('hex')],
            [SYSVAR_INSTRUCTIONS_PUBKEY],
            [],
            [],
            [instr]
        );

        expect(res["0"]).toBe(false);
    });
});
