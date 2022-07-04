import expect from 'expect';
import { publicKeyToHex } from '@solana/solidity';
import * as web3 from '@solana/web3.js';
import { loadContract } from './setup';

describe('Deploy solang contract and test', function () {
    this.timeout(500000);

    it('balances', async function () {
        let { contract, connection, payer, storage } = await loadContract('balances', 'balances.abi');

        let res = await contract.functions.get_balance(publicKeyToHex(payer.publicKey), {
            accounts: [payer.publicKey],
        });

        let bal = Number(res.result);

        let rpc_bal = await connection.getBalance(payer.publicKey);

        expect(bal + 5000).toBe(rpc_bal);

        // we wish to test the `.send()` function, so first top up the storage balance
        let before_bal = await connection.getBalance(storage.publicKey);

        /// transfer some lamports to the storage account
        var transaction = new web3.Transaction().add(
            web3.SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: storage.publicKey,
                lamports: 1500,
            }),
        );

        // Sign transaction, broadcast, and confirm
        await web3.sendAndConfirmTransaction(connection, transaction, [payer]);

        await contract.functions.send(publicKeyToHex(payer.publicKey), 500, {
            writableAccounts: [payer.publicKey],
            //  signers: [storage],
        });

        expect(await connection.getBalance(storage.publicKey)).toBe(before_bal + 1000);
    });
});
