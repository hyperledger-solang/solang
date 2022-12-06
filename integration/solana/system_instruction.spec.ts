// SPDX-License-Identifier: Apache-2.0

import { loadContract } from "./setup";
import { Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";


describe('Test system instructions', function () {
    this.timeout(500000);
    const system_account = new PublicKey('11111111111111111111111111111111');
    const recent_block_hashes = new PublicKey('SysvarRecentB1ockHashes11111111111111111111');
    const rentAddress = new PublicKey('SysvarRent111111111111111111111111111111111');
    const seed = 'my_seed_is_tea';

    it('create account', async function create_account() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction');
        const to_key_pair = Keypair.generate();

        await contract.functions.create_account(
            payer.publicKey.toBytes(),
            to_key_pair.publicKey.toBytes(),
            100000000,
            5,
            TOKEN_PROGRAM_ID.toBytes(),
            {
                accounts: [system_account, TOKEN_PROGRAM_ID],
                writableAccounts: [payer.publicKey, to_key_pair.publicKey],
                signers: [payer, to_key_pair],
            }
        );
    });

    it('create account with seed', async function create_account_with_seed() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction');
        const base_keypair = Keypair.generate();
        const to_key_pair = await PublicKey.createWithSeed(base_keypair.publicKey, seed, TOKEN_PROGRAM_ID);

        await contract.functions.create_account_with_seed(
            payer.publicKey.toBytes(),
            to_key_pair.toBytes(),
            base_keypair.publicKey.toBytes(),
            seed,
            100000000,
            5,
            TOKEN_PROGRAM_ID.toBytes(),
            {
                accounts: [system_account, TOKEN_PROGRAM_ID],
                writableAccounts: [payer.publicKey, to_key_pair],
                signers: [payer, base_keypair]
            }
        );
    });

    it('assign', async function assign() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction');
        const to_key_pair = Keypair.generate();

        const assign_account = new PublicKey('AddressLookupTab1e1111111111111111111111111');
        await contract.functions.assign(
            to_key_pair.publicKey.toBytes(),
            assign_account.toBytes(),
            {
                accounts: [system_account, payer.publicKey],
                writable_accounts: [to_key_pair.publicKey],
                signers: [to_key_pair],
            }
        )
    });

    it('assign with seed', async function assign_with_with_seed() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction');

        const assign_account = new PublicKey('AddressLookupTab1e1111111111111111111111111');
        const to_key_pair = await PublicKey.createWithSeed(payer.publicKey, seed, assign_account);

        await contract.functions.assign_with_seed(
            to_key_pair.toBytes(),
            payer.publicKey.toBytes(),
            seed,
            assign_account.toBytes(),
            {
                accounts: [system_account, assign_account],
                writableAccounts: [to_key_pair],
                signers: [payer]
            }
        );
    });

    it('transfer', async function transfer() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction');
        const dest = new Keypair();

        await contract.functions.transfer(
            payer.publicKey.toBytes(),
            dest.publicKey.toBytes(),
            100000000,
            {
                accounts: [system_account],
                writableAccounts: [payer.publicKey, dest.publicKey],
                signers: [payer]
            }
        );
    });

    it('transfer with seed', async function transfer_with_seed() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction');
        const dest = new Keypair();
        const assign_account = new PublicKey('AddressLookupTab1e1111111111111111111111111');
        const derived_payer = await PublicKey.createWithSeed(payer.publicKey, seed, assign_account);

        let signature = await connection.requestAirdrop(derived_payer, LAMPORTS_PER_SOL);
        await connection.confirmTransaction(signature, 'confirmed');

        await contract.functions.transfer_with_seed(
            derived_payer.toBytes(),
            payer.publicKey.toBytes(),
            seed,
            assign_account.toBytes(),
            dest.publicKey.toBytes(),
            100000000,
            {
                accounts: [system_account, assign_account],
                writableAccounts: [derived_payer, dest.publicKey],
                signers: [payer]
            }
        );
    });

    it('allocate', async function allocate() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction');
        const account = Keypair.generate();

        await contract.functions.allocate(
            account.publicKey.toBytes(),
            2,
            {
                accounts: [system_account],
                writableAccounts: [account.publicKey],
                signers: [account]
            }
        );
    });

    it('allocate with seed', async function allocate_with_seed() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction');
        const account = Keypair.generate();
        const owner = new PublicKey('Stake11111111111111111111111111111111111111');
        const derived_key = await PublicKey.createWithSeed(account.publicKey, seed, owner);

        await contract.functions.allocate_with_seed(
            derived_key.toBytes(),
            account.publicKey.toBytes(),
            seed,
            200,
            owner.toBytes(),
            {
                accounts: [system_account, owner],
                writableAccounts: [derived_key],
                signers: [account],
            }
        );
    });

    it('create nonce account with seed', async function create_nonce_account_with_seed() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction');
        const base_address = Keypair.generate();
        const derived_account = await PublicKey.createWithSeed(base_address.publicKey, seed, system_account);
        const authority = Keypair.generate();

        await contract.functions.create_nonce_account_with_seed(
            payer.publicKey.toBytes(),
            derived_account.toBytes(),
            base_address.publicKey.toBytes(),
            seed,
            authority.publicKey.toBytes(),
            100000000,
            {
                accounts: [system_account, recent_block_hashes, rentAddress],
                writableAccounts: [payer.publicKey, derived_account],
                signers: [payer, base_address]
            }
        );
    });

    it('nonce accounts', async function nonce_accounts() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction');
        const nonce = Keypair.generate();
        const authority = Keypair.generate();

        await contract.functions.create_nonce_account(
            payer.publicKey.toBytes(),
            nonce.publicKey.toBytes(),
            authority.publicKey.toBytes(),
            100000000,
            {
                accounts: [system_account, recent_block_hashes, rentAddress],
                writableAccounts: [payer.publicKey, nonce.publicKey],
                signers: [payer, nonce],
            }
        );

        await contract.functions.advance_nonce_account(
            nonce.publicKey.toBytes(),
            authority.publicKey.toBytes(),
            {
                accounts: [system_account, recent_block_hashes],
                writableAccounts: [nonce.publicKey],
                signers: [authority],
            }
        );

        await contract.functions.withdraw_nonce_account(
            nonce.publicKey.toBytes(),
            authority.publicKey.toBytes(),
            payer.publicKey.toBytes(),
            1000,
            {
                accounts: [system_account, recent_block_hashes, rentAddress],
                writableAccounts: [nonce.publicKey, payer.publicKey],
                signers: [authority],
            }
        );

        const new_authority = Keypair.generate();
        await contract.functions.authorize_nonce_account(
            nonce.publicKey.toBytes(),
            authority.publicKey.toBytes(),
            new_authority.publicKey.toBytes(),
            {
                accounts: [system_account],
                writableAccounts: [nonce.publicKey],
                signers: [authority],
            }
        );
    });
});
