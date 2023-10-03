// SPDX-License-Identifier: Apache-2.0

import { getOrCreateAssociatedTokenAccount, createMint, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { Keypair } from '@solana/web3.js';
import { loadContractAndCallConstructor } from './setup';
import { BN } from '@coral-xyz/anchor';
import expect from 'expect';

describe('Create spl-token and use from solidity', function () {
    this.timeout(500000);

    it('spl-token', async function name() {
        const { provider, storage, payer, program } = await loadContractAndCallConstructor('Token');
        const connection = provider.connection;

        const mintAuthority = Keypair.generate();
        const freezeAuthority = Keypair.generate();

        const mint = await createMint(
            connection,
            payer,
            mintAuthority.publicKey,
            freezeAuthority.publicKey,
            3
        );

        await program.methods.setMint(mint)
            .accounts({ dataAccount: storage.publicKey })
            .rpc();

        let total_supply = await program.methods.totalSupply()
            .accounts({
                dataAccount: storage.publicKey,
                mint: mint
            })
            .view();
        expect(total_supply.toNumber()).toBe(0);

        const tokenAccount = await getOrCreateAssociatedTokenAccount(
            connection,
            payer,
            mint,
            payer.publicKey
        )

        let balance = await program.methods.getBalance()
            .accounts({account: tokenAccount.address})
            .view();

        expect(balance.toNumber()).toBe(0);

        // Now let's mint some tokens
        await program.methods.mintTo(
            new BN(100000))
            .accounts({
                dataAccount: storage.publicKey,
                mint: mint,
                account: tokenAccount.address,
                authority: mintAuthority.publicKey,
            })
            .signers([mintAuthority])
            .rpc();

        // let's check the balances
        total_supply = await program.methods.totalSupply()
            .accounts({
                dataAccount: storage.publicKey,
                mint: mint,
            })
            .view();

        expect(total_supply.toNumber()).toBe(100000);

        balance = await program.methods.getBalance()
            .accounts({account: tokenAccount.address})
            .view();

        expect(balance.toNumber()).toBe(100000);

        // transfer
        const theOutsider = Keypair.generate();

        const otherTokenAccount = await getOrCreateAssociatedTokenAccount(
            connection,
            payer,
            mint,
            theOutsider.publicKey
        )

        await program.methods.transfer(
            new BN(70000))
            .accounts(
                {
                    from: tokenAccount.address,
                    to: otherTokenAccount.address,
                    owner: payer.publicKey
                }
            )
            .signers([payer])
            .rpc();

        total_supply = await program.methods.totalSupply()
            .accounts({
                dataAccount: storage.publicKey,
                mint: mint,
            })
            .view();

        expect(total_supply.toNumber()).toBe(100000);
        balance = await program.methods.getBalance()
            .accounts({account: tokenAccount.address})
            .view();

        expect(balance.toNumber()).toBe(30000);

        balance = await program.methods.getBalance()
            .accounts({account: otherTokenAccount.address})
            .view();

        expect(balance.toNumber()).toBe(70000);

        // burn
        await program.methods.burn(
            new BN(20000))
            .accounts({
                mint: mint,
                account: otherTokenAccount.address,
                owner: theOutsider.publicKey,
            })
            .signers([theOutsider])
            .rpc();

        total_supply = await program.methods.totalSupply()
            .accounts({
                dataAccount: storage.publicKey,
                mint: mint,
            })
            .view();

        expect(total_supply.toNumber()).toBe(80000);
        balance = await program.methods.getBalance()
            .accounts({account: tokenAccount.address})
            .view();

        expect(balance.toNumber()).toBe(30000);
        balance = await program.methods.getBalance()
            .accounts({account: otherTokenAccount.address})
            .view();

        expect(balance.toNumber()).toBe(50000);
    });
});