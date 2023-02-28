// SPDX-License-Identifier: Apache-2.0

// DISCLAIMER: This file is an example of how to mint and transfer NFTs on Solana. It is not production ready and has not been audited for security.
// Use it at your own risk.

import { loadContract, newConnectionAndPayer } from "./setup";
import { Keypair } from "@solana/web3.js";
import { createMint, getOrCreateAssociatedTokenAccount, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import expect from "expect";

describe('Simple collectible', function () {
    this.timeout(500000);

    it('nft example', async function mint_nft() {
        const [connection, payer] = newConnectionAndPayer();
        const mint_authority = Keypair.generate();
        const freezeAuthority = Keypair.generate();

        // Create and initialize a new mint based on the funding account and a mint authority
        const mint = await createMint(
            connection,
            payer,
            mint_authority.publicKey,
            freezeAuthority.publicKey,
            0
        );

        const nft_owner = Keypair.generate();
        const metadata_authority = Keypair.generate();

        // On Solana, an account must have an associated token account to save information about how many tokens
        // the owner account owns. The associated account depends on both the mint account and the owner
        const owner_token_account = await getOrCreateAssociatedTokenAccount(
            connection,
            payer,
            mint, // Mint account
            nft_owner.publicKey // Owner account
        );

        // Each contract in this example is a unique NFT
        const { provider, program, storage } = await loadContract('SimpleCollectible', [mint, metadata_authority.publicKey]);

        const nft_uri = "www.nft.com";

        // Create a collectible for an owner given a mint authority.
        await program.methods.createCollectible(
            nft_uri,
            mint_authority.publicKey,
            owner_token_account.address)
            .accounts({ dataAccount: storage.publicKey })
            .remainingAccounts([
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
                { pubkey: mint, isSigner: false, isWritable: true },
                { pubkey: owner_token_account.address, isSigner: false, isWritable: true },
                { pubkey: mint_authority.publicKey, isSigner: true, isWritable: true },
                { pubkey: metadata_authority.publicKey, isSigner: true, isWritable: true }
            ])
            .signers([mint_authority, metadata_authority])
            .rpc();

        const new_owner = Keypair.generate();

        // A new owner must have an associated token account
        const new_owner_token_account = await getOrCreateAssociatedTokenAccount(
            connection,
            payer,
            mint, // Mint account associated to the NFT
            new_owner.publicKey // New owner account
        );


        // Transfer ownership to another owner
        await program.methods.transferOwnership(
            owner_token_account.address,
            new_owner_token_account.address)
            .accounts({ dataAccount: storage.publicKey })
            .remainingAccounts([
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
                { pubkey: new_owner_token_account.address, isSigner: false, isWritable: true },
                { pubkey: owner_token_account.address, isSigner: false, isWritable: true },
                { pubkey: nft_owner.publicKey, isSigner: true, isWritable: true },
            ])
            .signers([nft_owner])
            .rpc();

        // Confirm that the ownership transference worked
        const verify_transfer_result = await program.methods.isOwner(
            new_owner.publicKey,
            new_owner_token_account.address)
            .accounts({ dataAccount: storage.publicKey })
            .remainingAccounts([
                { pubkey: new_owner_token_account.address, isSigner: false, isWritable: false },
            ])
            .view();

        expect(verify_transfer_result).toBe(true);

        // Retrieve information about the NFT
        const token_uri = await program.methods.getNftUri()
            .accounts({ dataAccount: storage.publicKey })
            .view();

        expect(token_uri).toBe(nft_uri);

        // Update the NFT URI
        const new_uri = "www.token.com";
        await program.methods.updateNftUri(new_uri)
            .accounts({ dataAccount: storage.publicKey })
            .remainingAccounts([
                { pubkey: metadata_authority.publicKey, isSigner: true, isWritable: true },
            ])
            .signers([metadata_authority])
            .rpc();

        const new_uri_saved = await program.methods.getNftUri()
            .accounts({ dataAccount: storage.publicKey })
            .view();
        expect(new_uri_saved).toBe(new_uri);
    });
});
