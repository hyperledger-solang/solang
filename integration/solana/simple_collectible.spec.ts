// SPDX-License-Identifier: Apache-2.0

// DISCLAIMER: This file is an example of how to mint and transfer NFTs on Solana. It is not production ready and has not been audited for security.
// Use it at your own risk.

import {loadContract} from "./setup";
import {Keypair, PublicKey, SystemProgram} from "@solana/web3.js";
import {publicKeyToHex, HexToPublicKey} from "@solana/solidity";
import {createMint, getOrCreateAssociatedTokenAccount, TOKEN_PROGRAM_ID} from "@solana/spl-token";

describe('Simple collectible', function() {
    this.timeout(500000);


    it('nft example', async function mint_nft() {
        const mint_authority = Keypair.generate();
        const { contract, connection, payer, program } = await loadContract('SimpleCollectible', 'SimpleCollectible.abi', [publicKeyToHex(mint_authority.publicKey)]);

        // Save the payer in Solidity
        await contract.functions.set_payer(
            publicKeyToHex(payer.publicKey)
        );

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
        const owner = await getOrCreateAssociatedTokenAccount(
            connection,
            payer,
            mint, // Mint account
            payer.publicKey // Owner account
        );

        // Create a collectible for an owner given a mint account. Each new NFT requires a new mint account.
        const nft_id = await contract.functions.createCollectible(
            "www.nft.com",
            publicKeyToHex(mint),
            publicKeyToHex(owner.address),
            {
                accounts: [TOKEN_PROGRAM_ID],
                writableAccounts: [mint, owner.address],
                signers: [mint_authority]
            }
        );

        // Retrieve the owner of an NFT
        const response = await contract.functions.get_nft_owner(
            nft_id.result
        );

        const new_owner = Keypair.generate();
        const existing_mint = new PublicKey(HexToPublicKey(response.result[1]));

        // A new owner must have an associated token account
        const associated_owner_account = await getOrCreateAssociatedTokenAccount(
            connection,
            payer,
            existing_mint, // Mint account associated to the NFT
            new_owner.publicKey // New owner account
        );

        const current_owner = new PublicKey(HexToPublicKey(response.result[0]));

        // Transfer ownership to another owner
        await contract.functions.transfer_ownership(
            nft_id.result,
            publicKeyToHex(associated_owner_account.address),
            {
                accounts: [TOKEN_PROGRAM_ID],
                writableAccounts: [current_owner, associated_owner_account.address],
                signers: [payer]
            }
        );

        // Retrieve information about the NFT
        const token_uri = await contract.functions.get_nft_uri(nft_id.result);
        console.log(token_uri.result);
    });
})