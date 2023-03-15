// SPDX-License-Identifier: Apache-2.0

import { loadContract } from "./setup";
import { Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { BN } from '@project-serum/anchor';


describe('Test system instructions', function () {
    this.timeout(500000);
    const system_account = new PublicKey('11111111111111111111111111111111');
    const recent_block_hashes = new PublicKey('SysvarRecentB1ockHashes11111111111111111111');
    const rentAddress = new PublicKey('SysvarRent111111111111111111111111111111111');
    const seed = 'my_seed_is_tea';

    it('create account', async function create_account() {
        const { program, storage, payer } = await loadContract('TestingInstruction');
        const to_key_pair = Keypair.generate();

        await program.methods.createAccount(
            payer.publicKey,
            to_key_pair.publicKey,
            new BN(100000000),
            new BN(5),
            TOKEN_PROGRAM_ID)
            .remainingAccounts([
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
                { pubkey: payer.publicKey, isSigner: true, isWritable: false },
                { pubkey: to_key_pair.publicKey, isSigner: true, isWritable: true },
            ])
            .accounts({ dataAccount: storage.publicKey })
            .signers([payer, to_key_pair]).rpc();
    });

    it('create account with seed', async function create_account_with_seed() {
        const { storage, payer, program } = await loadContract('TestingInstruction');
        const base_keypair = Keypair.generate();
        const to_key_pair = await PublicKey.createWithSeed(base_keypair.publicKey, seed, TOKEN_PROGRAM_ID);

        await program.methods.createAccountWithSeed(
            payer.publicKey,
            to_key_pair,
            base_keypair.publicKey,
            seed,
            new BN(100000000),
            new BN(5),
            TOKEN_PROGRAM_ID)
            .remainingAccounts([
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
                { pubkey: payer.publicKey, isSigner: true, isWritable: false },
                { pubkey: to_key_pair, isSigner: false, isWritable: true },
                { pubkey: base_keypair.publicKey, isSigner: true, isWritable: false },
            ])
            .accounts({ dataAccount: storage.publicKey })
            .signers([payer, base_keypair]).rpc();
    });

    it('assign', async function assign() {
        const { storage, payer, program } = await loadContract('TestingInstruction');
        const to_key_pair = Keypair.generate();

        const assign_account = new PublicKey('AddressLookupTab1e1111111111111111111111111');
        await program.methods.assign(
            to_key_pair.publicKey,
            assign_account)
            .remainingAccounts([
                { pubkey: payer.publicKey, isSigner: false, isWritable: false },
                { pubkey: to_key_pair.publicKey, isSigner: true, isWritable: true },
            ])
            .accounts({ dataAccount: storage.publicKey })
            .signers([payer, to_key_pair]).rpc();
    });

    it('assign with seed', async function assign_with_with_seed() {
        const { storage, payer, program } = await loadContract('TestingInstruction');
        const assign_account = new PublicKey('AddressLookupTab1e1111111111111111111111111');
        const to_key_pair = await PublicKey.createWithSeed(payer.publicKey, seed, assign_account);

        await program.methods.assignWithSeed(
            to_key_pair,
            payer.publicKey,
            seed,
            assign_account)
            .remainingAccounts([
                { pubkey: assign_account, isSigner: false, isWritable: false },
                { pubkey: payer.publicKey, isSigner: false, isWritable: false },
                { pubkey: to_key_pair, isSigner: false, isWritable: true },
            ])
            .accounts({ dataAccount: storage.publicKey })
            .signers([payer]).rpc();
    });

    it('transfer', async function transfer() {
        const { storage, payer, program } = await loadContract('TestingInstruction');
        const dest = new Keypair();

        await program.methods.transfer(
            payer.publicKey,
            dest.publicKey,
            new BN(100000000))
            .remainingAccounts([
                { pubkey: payer.publicKey, isSigner: false, isWritable: true },
                { pubkey: dest.publicKey, isSigner: false, isWritable: true },
            ])
            .accounts({ dataAccount: storage.publicKey })
            .signers([payer]).rpc();
    });

    it('transfer with seed', async function transfer_with_seed() {
        const { storage, payer, provider, program } = await loadContract('TestingInstruction');
        const dest = new Keypair();
        const assign_account = new PublicKey('AddressLookupTab1e1111111111111111111111111');
        const derived_payer = await PublicKey.createWithSeed(payer.publicKey, seed, assign_account);

        let signature = await provider.connection.requestAirdrop(derived_payer, LAMPORTS_PER_SOL);
        await provider.connection.confirmTransaction(signature, 'confirmed');

        await program.methods.transferWithSeed(
            derived_payer, // from_pubkey
            payer.publicKey, // from_base
            seed, // seed
            assign_account, // from_owner
            dest.publicKey, // to_pubkey
            new BN(100000000))
            .remainingAccounts([
                { pubkey: assign_account, isSigner: false, isWritable: false },
                { pubkey: derived_payer, isSigner: false, isWritable: true },
                { pubkey: dest.publicKey, isSigner: false, isWritable: true },
                { pubkey: payer.publicKey, isSigner: true, isWritable: false },
            ])
            .accounts({ dataAccount: storage.publicKey })
            .signers([payer]).rpc();
    });

    it('allocate', async function allocate() {
        const { storage, payer, program } = await loadContract('TestingInstruction');
        const account = Keypair.generate();

        await program.methods.allocate(
            account.publicKey,
            new BN(2))
            .remainingAccounts([
                { pubkey: account.publicKey, isSigner: true, isWritable: true },
            ])
            .accounts({ dataAccount: storage.publicKey })
            .signers([payer, account]).rpc();
    });

    it('allocate with seed', async function allocate_with_seed() {
        const { storage, payer, program } = await loadContract('TestingInstruction');
        const account = Keypair.generate();
        const owner = new PublicKey('Stake11111111111111111111111111111111111111');
        const derived_key = await PublicKey.createWithSeed(account.publicKey, seed, owner);

        await program.methods.allocateWithSeed(
            derived_key,
            account.publicKey,
            seed,
            new BN(200),
            owner)
            .remainingAccounts([
                { pubkey: owner, isSigner: false, isWritable: false },
                { pubkey: account.publicKey, isSigner: true, isWritable: false },
                { pubkey: derived_key, isSigner: false, isWritable: true },
            ])
            .accounts({ dataAccount: storage.publicKey })
            .signers([payer, account]).rpc();
    });

    it('create nonce account with seed', async function create_nonce_account_with_seed() {
        const { storage, payer, program } = await loadContract('TestingInstruction');
        const base_address = Keypair.generate();
        const derived_account = await PublicKey.createWithSeed(base_address.publicKey, seed, system_account);
        const authority = Keypair.generate();

        await program.methods.createNonceAccountWithSeed(
            payer.publicKey,
            derived_account,
            base_address.publicKey,
            seed,
            authority.publicKey,
            new BN(100000000))
            .remainingAccounts([
                { pubkey: recent_block_hashes, isSigner: false, isWritable: false },
                { pubkey: rentAddress, isSigner: false, isWritable: false },
                { pubkey: payer.publicKey, isSigner: false, isWritable: true },
                { pubkey: derived_account, isSigner: false, isWritable: true },
                { pubkey: base_address.publicKey, isSigner: true, isWritable: true },
            ])
            .accounts({ dataAccount: storage.publicKey })
            .signers([payer, base_address]).rpc();
    });

    it('nonce accounts', async function nonce_accounts() {
        const { storage, payer, program } = await loadContract('TestingInstruction');
        const nonce = Keypair.generate();
        const authority = Keypair.generate();

        await program.methods.createNonceAccount(
            payer.publicKey,
            nonce.publicKey,
            authority.publicKey,
            new BN(100000000))
            .remainingAccounts([
                { pubkey: recent_block_hashes, isSigner: false, isWritable: false },
                { pubkey: rentAddress, isSigner: false, isWritable: false },
                { pubkey: payer.publicKey, isSigner: false, isWritable: true },
                { pubkey: nonce.publicKey, isSigner: true, isWritable: true },
            ])
            .accounts({ dataAccount: storage.publicKey })
            .signers([payer, nonce]).rpc();

        await program.methods.advanceNonceAccount(
            nonce.publicKey,
            authority.publicKey)
            .remainingAccounts([
                { pubkey: recent_block_hashes, isSigner: false, isWritable: false },
                { pubkey: authority.publicKey, isSigner: true, isWritable: false },
                { pubkey: nonce.publicKey, isSigner: false, isWritable: true },
            ])
            .accounts({ dataAccount: storage.publicKey })
            .signers([authority]).rpc();

        await program.methods.withdrawNonceAccount(
            nonce.publicKey,
            authority.publicKey,
            payer.publicKey,
            new BN(1000))
            .remainingAccounts([
                { pubkey: recent_block_hashes, isSigner: false, isWritable: false },
                { pubkey: rentAddress, isSigner: false, isWritable: false },
                { pubkey: authority.publicKey, isSigner: true, isWritable: false },
                { pubkey: nonce.publicKey, isSigner: false, isWritable: true },
                { pubkey: payer.publicKey, isSigner: false, isWritable: true },
            ])
            .accounts({ dataAccount: storage.publicKey })
            .signers([authority]).rpc();

        const new_authority = Keypair.generate();
        await program.methods.authorizeNonceAccount(
            nonce.publicKey,
            authority.publicKey,
            new_authority.publicKey)

            .remainingAccounts([
                { pubkey: authority.publicKey, isSigner: true, isWritable: false },
                { pubkey: nonce.publicKey, isSigner: false, isWritable: true },
            ])
            .accounts({ dataAccount: storage.publicKey })
            .signers([authority]).rpc();
    });
});
