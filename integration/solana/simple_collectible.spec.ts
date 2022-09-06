// SPDX-License-Identifier: Apache-2.0

// DISCLAIMER: This file is an example of how to mint and transfer NFTs on Solana. It is not production ready and has not been audited for security.
// Use it at your own risk.

import {loadContractWithExistingConnectionAndPayer, loadContract, newConnectionAndAccounts} from "./setup";
import {Connection, Keypair, PublicKey, SystemProgram} from "@solana/web3.js";
import {publicKeyToHex, HexToPublicKey} from "@solana/solidity";
import {createMint, getOrCreateAssociatedTokenAccount, TOKEN_PROGRAM_ID} from "@solana/spl-token";
import expect from "expect";

describe('Simple collectible', function() {
    this.timeout(500000);

    it('nft example', async function mint_nft() {
        const [connection, payer, program] = newConnectionAndAccounts();
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
        const contract = await loadContractWithExistingConnectionAndPayer(
            connection,
            program,
            payer,
            "SimpleCollectible",
            "SimpleCollectible.abi",
            [publicKeyToHex(mint), publicKeyToHex(metadata_authority.publicKey)]
        );

        const nft_uri = "www.nft.com";

        // Create a collectible for an owner given a mint authority.
        await contract.functions.createCollectible(
            nft_uri,
            publicKeyToHex(mint_authority.publicKey),
            publicKeyToHex(owner_token_account.address),
            {
                accounts: [TOKEN_PROGRAM_ID],
                writableAccounts: [mint, owner_token_account.address],
                signers: [mint_authority, metadata_authority]
            }
        );

        const new_owner = Keypair.generate();

        // A new owner must have an associated token account
        const new_owner_token_account = await getOrCreateAssociatedTokenAccount(
            connection,
            payer,
            mint, // Mint account associated to the NFT
            new_owner.publicKey // New owner account
        );


        // Transfer ownership to another owner
        await contract.functions.transferOwnership(
            publicKeyToHex(owner_token_account.address),
            publicKeyToHex(new_owner_token_account.address),
            {
                accounts: [TOKEN_PROGRAM_ID],
                writableAccounts: [owner_token_account.address, new_owner_token_account.address],
                signers: [nft_owner]
            }
        );

        // Confirm that the ownership transference worked
        const verify_transfer_result = await contract.functions.isOwner(
            publicKeyToHex(new_owner.publicKey),
            publicKeyToHex(new_owner_token_account.address),
            {
                accounts: [new_owner_token_account.address],
            }
        );

        expect(verify_transfer_result.result).toBe(true);

        // Retrieve information about the NFT
        const token_uri = await contract.functions.getNftUri();
        expect(token_uri.result).toBe(nft_uri);

        // Update the NFT URI
        const new_uri = "www.token.com";
        await contract.functions.updateNftUri(
            new_uri,
            {
                signers: [metadata_authority]
            }
        );

        const new_uri_saved = await contract.functions.getNftUri();
        expect(new_uri_saved.result).toBe(new_uri);
    });
});
