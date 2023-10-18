// SPDX-License-Identifier: Apache-2.0

import { loadContractAndCallConstructor } from "./setup";
import { Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { BN } from '@coral-xyz/anchor';


describe('Test system instructions', function () {
    this.timeout(500000);
    const system_account = new PublicKey('11111111111111111111111111111111');
    const recent_block_hashes = new PublicKey('SysvarRecentB1ockHashes11111111111111111111');
    const rentAddress = new PublicKey('SysvarRent111111111111111111111111111111111');
    const seed = 'my_seed_is_tea';

    it('create account', async function create_account() {
        const { program, storage, payer } = await loadContractAndCallConstructor('TestingInstruction');
        const to_key_pair = Keypair.generate();

        await program.methods.createAccount(
            new BN(100000000),
            new BN(5),
            TOKEN_PROGRAM_ID
        ).accounts(
                {
                    from: payer.publicKey,
                    to: to_key_pair.publicKey
                }
            )
            .remainingAccounts([
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
            ])
            .signers([payer, to_key_pair]).rpc();
    });

    it('create account with seed', async function create_account_with_seed() {
        const { payer, program } = await loadContractAndCallConstructor('TestingInstruction');
        const base_keypair = Keypair.generate();
        const to_key_pair = await PublicKey.createWithSeed(base_keypair.publicKey, seed, TOKEN_PROGRAM_ID);

        await program.methods.createAccountWithSeed(
            seed,
            new BN(100000000),
            new BN(5),
            TOKEN_PROGRAM_ID)
            .accounts({
                from: payer.publicKey,
                to: to_key_pair,
                base: base_keypair.publicKey
            })
            .remainingAccounts([
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
            ])
            .signers([payer, base_keypair]).rpc();
    });

    it('assign', async function assign() {
        const { program } = await loadContractAndCallConstructor('TestingInstruction');
        const to_key_pair = Keypair.generate();

        const assign_account = new PublicKey('AddressLookupTab1e1111111111111111111111111');
        await program.methods.assign(
            assign_account)
            .accounts(
                {
                    assignAccount: to_key_pair.publicKey,
                }
            )
            .signers([to_key_pair]).rpc();
    });

    it('assign with seed', async function assign_with_with_seed() {
        const { storage, payer, program } = await loadContractAndCallConstructor('TestingInstruction');
        const assign_account = new PublicKey('AddressLookupTab1e1111111111111111111111111');
        const to_key_pair = await PublicKey.createWithSeed(payer.publicKey, seed, assign_account);

        await program.methods.assignWithSeed(
            seed,
            assign_account)
            .accounts({
                assignAccount: to_key_pair,
                owner: payer.publicKey,
            })
            .signers([payer]).rpc();
    });

    it('transfer', async function transfer() {
        const { storage, payer, program } = await loadContractAndCallConstructor('TestingInstruction');
        const dest = new Keypair();

        await program.methods.transfer(
            new BN(100000000))
            .accounts({
                from: payer.publicKey,
                to: dest.publicKey,
            })
            .signers([payer]).rpc();
    });

    it('transfer with seed', async function transfer_with_seed() {
        const { payer, provider, program } = await loadContractAndCallConstructor('TestingInstruction');
        const dest = new Keypair();
        const assign_account = new PublicKey('AddressLookupTab1e1111111111111111111111111');
        const derived_payer = await PublicKey.createWithSeed(payer.publicKey, seed, assign_account);

        let signature = await provider.connection.requestAirdrop(derived_payer, LAMPORTS_PER_SOL);
        await provider.connection.confirmTransaction(signature, 'confirmed');

        await program.methods.transferWithSeed(
            seed, // seed
            assign_account, // from_owner
            new BN(100000000))
            .accounts(
                {
                    fromKey: derived_payer,
                    fromBase: payer.publicKey,
                    toKey: dest.publicKey,
                }
            )
            .signers([payer]).rpc({commitment: "confirmed"});
    });

    it('allocate', async function allocate() {
        const {  program } = await loadContractAndCallConstructor('TestingInstruction');
        const account = Keypair.generate();

        await program.methods.allocate(
            new BN(2))
            .accounts({accKey: account.publicKey})
            .signers([account]).rpc();
    });

    it('allocate with seed', async function allocate_with_seed() {
        const { storage, payer, program } = await loadContractAndCallConstructor('TestingInstruction');
        const account = Keypair.generate();
        const owner = new PublicKey('Stake11111111111111111111111111111111111111');
        const derived_key = await PublicKey.createWithSeed(account.publicKey, seed, owner);

        await program.methods.allocateWithSeed(
            seed,
            new BN(200),
            owner)
            .accounts({
                accKey: derived_key,
                base: account.publicKey,
            })
            .signers([account]).rpc();
    });

    it('create nonce account with seed', async function create_nonce_account_with_seed() {
        const { storage, payer, program } = await loadContractAndCallConstructor('TestingInstruction');
        const base_address = Keypair.generate();
        const derived_account = await PublicKey.createWithSeed(base_address.publicKey, seed, system_account);
        const authority = Keypair.generate();

        await program.methods.createNonceAccountWithSeed(
            seed,
            authority.publicKey,
            new BN(100000000))
            .accounts(
                {from: payer.publicKey,
                nonce: derived_account,
                base: base_address.publicKey}
            )
            .remainingAccounts([
                { pubkey: recent_block_hashes, isSigner: false, isWritable: false },
                { pubkey: rentAddress, isSigner: false, isWritable: false },
            ])
            .signers([payer, base_address]).rpc({commitment: "confirmed"});
    });

    it('nonce accounts', async function nonce_accounts() {
        const { storage, payer, program } = await loadContractAndCallConstructor('TestingInstruction');
        const nonce = Keypair.generate();
        const authority = Keypair.generate();

        await program.methods.createNonceAccount(
            authority.publicKey,
            new BN(100000000))
            .accounts(
                {
                    from: payer.publicKey,
                    nonce: nonce.publicKey
                }
            )
            .remainingAccounts([
                { pubkey: recent_block_hashes, isSigner: false, isWritable: false },
                { pubkey: rentAddress, isSigner: false, isWritable: false },
            ])
            .signers([payer, nonce]).rpc();

        await program.methods.advanceNonceAccount(authority.publicKey)
            .accounts({nonce: nonce.publicKey})
            .remainingAccounts([
                { pubkey: recent_block_hashes, isSigner: false, isWritable: false },
                { pubkey: authority.publicKey, isSigner: true, isWritable: false },
            ])
            .signers([authority]).rpc();

        await program.methods.withdrawNonceAccount(
            new BN(1000))
            .accounts({
                nonce: nonce.publicKey,
                to: payer.publicKey,
                authority: authority.publicKey
            })
            .remainingAccounts([
                { pubkey: recent_block_hashes, isSigner: false, isWritable: false },
                { pubkey: rentAddress, isSigner: false, isWritable: false },
            ])
            .signers([authority]).rpc();

        const new_authority = Keypair.generate();
        await program.methods.authorizeNonceAccount(
            new_authority.publicKey)
            .accounts({
                nonce: nonce.publicKey,
                authority: authority.publicKey
            })
            .signers([authority]).rpc();
    });
});
