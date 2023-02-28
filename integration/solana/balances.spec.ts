// SPDX-License-Identifier: Apache-2.0

import expect from 'expect';
import { Transaction, SystemProgram, sendAndConfirmTransaction } from '@solana/web3.js';
import { loadContract } from './setup';
import { BN } from '@project-serum/anchor';

describe('Deploy solang contract and test', function () {
    this.timeout(500000);

    it('balances', async function () {
        let { program, storage, payer, provider } = await loadContract('balances', []);

        let res = await program.methods.getBalance(payer.publicKey)
            .accounts({ dataAccount: storage.publicKey })
            .remainingAccounts([{ pubkey: payer.publicKey, isSigner: false, isWritable: false }])
            .view();

        let bal = Number(res);

        let rpc_bal = await provider.connection.getBalance(payer.publicKey);

        expect(bal + 5000).toBe(rpc_bal);

        // we wish to test the `.send()` function, so first top up the storage balance
        let before_bal = await provider.connection.getBalance(storage.publicKey);

        /// transfer some lamports to the storage account
        const transaction = new Transaction().add(
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: storage.publicKey,
                lamports: 1500,
            }),
        );

        // Sign transaction, broadcast, and confirm
        await sendAndConfirmTransaction(provider.connection, transaction, [payer]);

        await program.methods.send(payer.publicKey, new BN(500))
            .accounts({ dataAccount: storage.publicKey })
            .remainingAccounts([
                { pubkey: storage.publicKey, isSigner: true, isWritable: true },
                { pubkey: payer.publicKey, isSigner: false, isWritable: true }
            ])
            .signers([storage])
            .rpc();

        expect(await provider.connection.getBalance(storage.publicKey)).toBe(before_bal + 1000);
    });
});
