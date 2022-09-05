// SPDX-License-Identifier: Apache-2.0

// DISCLAIMER: This file is an example of how to mint and transfer NFTs on Solana. It is not production ready and has not been audited for security.
// Use it at your own risk.

import {loadContract} from "./setup";
import {Keypair, PublicKey, SystemProgram} from "@solana/web3.js";
import {publicKeyToHex, HexToPublicKey} from "@solana/solidity";
import {createMint, getOrCreateAssociatedTokenAccount, TOKEN_PROGRAM_ID} from "@solana/spl-token";
import expect from "expect";

describe('Simple collectible', function() {
    this.timeout(500000);


    it('nft example', async function mint_nft() {
        const mint_authority = Keypair.generate();
        const { contract, connection, payer, program } = await loadContract('SimpleCollectible', 'SimpleCollectible.abi', [publicKeyToHex(mint_authority.publicKey)]);


        const freezeAuthority = Keypair.generate();
        // Create and initialize a new mint based on the funding account and a mint authority
        const mint = await createMint(
            connection,
            payer,
            mint_authority.publicKey,
            freezeAuthority.publicKey,
            3
        );

        // On Solana, an account must have an associated token account to save information about how many tokens
        // the owner account owns. The associated account depends on both the mint account and the owner
        const owner_token_account = await getOrCreateAssociatedTokenAccount(
            connection,
            payer,
            mint, // Mint account
            payer.publicKey // Owner account
        );

        const nft_uri = "www.nft.com";

        // Create a collectible for an owner given a mint account. Each new NFT requires a new mint account.
        const nft_id = await contract.functions.createCollectible(
            nft_uri,
            publicKeyToHex(mint),
            publicKeyToHex(payer.publicKey),
            publicKeyToHex(owner_token_account.address),
            {
                accounts: [TOKEN_PROGRAM_ID],
                writableAccounts: [mint, owner_token_account.address],
                signers: [mint_authority]
            }
        );

        // Retrieve the owner of an NFT
        const response = await contract.functions.getNftOwner(
            nft_id.result
        );
        expect(response.result[0]).toBe(true);

        const new_owner = Keypair.generate();
        const existing_mint = new PublicKey(HexToPublicKey(response.result[3]));

        // A new owner must have an associated token account
        const new_owner_token_account = await getOrCreateAssociatedTokenAccount(
            connection,
            payer,
            existing_mint, // Mint account associated to the NFT
            new_owner.publicKey // New owner account
        );

        const current_owner_token_account = new PublicKey(HexToPublicKey(response.result[2]));

        // Transfer ownership to another owner
        await contract.functions.transferOwnership(
            nft_id.result,
            publicKeyToHex(new_owner.publicKey),
            publicKeyToHex(new_owner_token_account.address),
            {
                accounts: [TOKEN_PROGRAM_ID],
                writableAccounts: [current_owner_token_account, new_owner_token_account.address],
                signers: [payer]
            }
        );

        const verify_transfer_result = await contract.functions.getNftOwner(
            nft_id.result
        );

        expect(verify_transfer_result.result[0]).toBe(true);
        expect(verify_transfer_result.result[1]).toBe(publicKeyToHex(new_owner.publicKey));
        expect(verify_transfer_result.result[2]).toBe(publicKeyToHex(new_owner_token_account.address));

        // Retrieve information about the NFT
        const token_uri = await contract.functions.getNftUri(nft_id.result);
        expect(token_uri.result).toBe(nft_uri);
    });
});
