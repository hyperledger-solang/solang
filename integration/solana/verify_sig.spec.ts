import { Contract, publicKeyToHex } from '@solana/solidity';
import { Keypair } from '@solana/web3.js';
import expect from 'expect';
import nacl from 'tweetnacl';
import { loadContract } from './setup';

describe('Signature Check', function () {
    this.timeout(150000);

    let contract: Contract;
    let payer: Keypair;

    before(async function () {
        ({ contract, payer } = await loadContract('verify_sig', 'verify_sig.abi'));
    });

    it('check valid signature', async function () {
        const message = Buffer.from('Foobar');
        const signature = nacl.sign.detached(message, payer.secretKey);

        const { result } = await contract.functions.verify(
            payer.publicKey.toBytes(), message, signature,
            {
                ed25519sigs: [{ publicKey: payer.publicKey, message, signature }],
            }
        );

        expect(result).toEqual(true);
    });

    it('check invalid signature', async function () {
        const message = Buffer.from('Foobar');
        const signature = nacl.sign.detached(message, payer.secretKey);

        const broken_signature = Buffer.from(signature);

        broken_signature[1] ^= 1;

        const { result } = await contract.functions.verify(
            payer.publicKey.toBytes(), message, broken_signature,
            {
                ed25519sigs: [{ publicKey: payer.publicKey, message, signature }],
            }
        );

        expect(result).toEqual(false);
    });
});
