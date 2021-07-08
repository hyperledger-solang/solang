import expect from 'expect';
import { establishConnection } from './index';
import nacl from 'tweetnacl';

describe('Deploy solang contract and test', () => {
    it('verify_signature', async function () {
        this.timeout(50000);

        // This test depends on: https://github.com/solana-labs/solana/pull/18328
        this.skip();

        let conn = await establishConnection();

        let prog = await conn.loadProgram("bundle.so", "verify_sig.abi");

        let message = Buffer.from('In the temple of love you hide together');

        let signature = nacl.sign.detached(message, prog.contractStorageAccount.secretKey);

        // call the constructor
        await prog.call_constructor(conn, 'verify_sig', []);

        let res = await prog.call_function(conn, "verify", ['0x' + prog.contractStorageAccount.publicKey.toBuffer().toString('hex'), '0x' + message.toString('hex'), '0x' + Buffer.from(signature).toString('hex')]);

        expect(res["0"]).toBe(true);

        signature[2] ^= 0x40;

        let res2 = await prog.call_function(conn, "verify", ['0x' + prog.contractStorageAccount.publicKey.toBuffer().toString('hex'), '0x' + message.toString('hex'), '0x' + Buffer.from(signature).toString('hex')]);

        expect(res2["0"]).toBe(false);
    });
});