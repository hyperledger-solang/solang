// SPDX-License-Identifier: Apache-2.0

import expect from 'expect';
import {Transaction, SystemProgram, sendAndConfirmTransaction, Keypair, LAMPORTS_PER_SOL} from '@solana/web3.js';
import { loadContractAndCallConstructor } from './setup';
import {BN} from '@coral-xyz/anchor';

describe('Deploy solang contract and test', function () {
    this.timeout(500000);

    it('balances', async function () {
        let { program, storage, payer, provider } = await loadContractAndCallConstructor('balances', []);

        let res = await program.methods.getBalance()
            .accounts({
                acc1: payer.publicKey
            })
            .view();

        let bal = Number(res);

        let rpc_bal = await provider.connection.getBalance(payer.publicKey);

        expect(bal + 5000).toBe(rpc_bal);

        /// transfer some lamports to the storage account
        let transaction = new Transaction().add(
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: storage.publicKey,
                lamports: LAMPORTS_PER_SOL,
            }),
        );

        // Sign transaction, broadcast, and confirm
        await sendAndConfirmTransaction(provider.connection, transaction, [payer]);

        const new_account = Keypair.generate();
        const lamports = await provider.connection.getMinimumBalanceForRentExemption(100);

        await program.methods.transfer(new BN(lamports))
            .accounts({
                acc1: storage.publicKey,
                acc2: new_account.publicKey,
            })
            .rpc();

        expect(await provider.connection.getBalance(new_account.publicKey)).toBe(lamports);
    });
});
