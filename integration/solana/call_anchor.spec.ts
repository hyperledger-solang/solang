import expect from 'expect';
import { loadContract } from './setup';
import { publicKeyToHex } from '@solana/solidity';
import { SystemProgram, PublicKey, Keypair } from '@solana/web3.js';


describe('Call Anchor program from Solidity via IDL', function () {
    this.timeout(500000);

    it('call_anchor', async function () {
        // This program instantiates an anchor program, calls various functions on it and checks the return values

        const data = Keypair.generate();

        const programId = new PublicKey("z7FbDfQDfucxJz5o8jrGLgvSbdoeSqX5VrxBb5TVjHq");

        let { contract, payer } = await loadContract('call_anchor', 'call_anchor.abi', [publicKeyToHex(data.publicKey)]);

        let { result } = await contract.functions.test(publicKeyToHex(payer.publicKey), { accounts: [programId, SystemProgram.programId], signers: [data, payer] });

        expect(result.toNumber()).toEqual(11);
    });
});
