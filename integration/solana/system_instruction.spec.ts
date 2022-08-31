// SPDX-License-Identifier: Apache-2.0

import {loadContract} from "./setup";
import {Keypair, LAMPORTS_PER_SOL, PublicKey} from "@solana/web3.js";
import {publicKeyToHex} from "@solana/solidity";
import {TOKEN_PROGRAM_ID} from "@solana/spl-token";


describe('Test system instructions', function() {
    this.timeout(500000);
    const system_account = new PublicKey('11111111111111111111111111111111');
    const recent_block_hashes = new PublicKey('SysvarRecentB1ockHashes11111111111111111111');
    const rentAddress = new PublicKey('SysvarRent111111111111111111111111111111111');
    const seed = 'my_seed_is_tea';

    it('create account', async function create_account() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction', 'TestingInstruction.abi');
        const to_key_pair = Keypair.generate();

        await contract.functions.create_account(
            publicKeyToHex(payer.publicKey),
            publicKeyToHex(to_key_pair.publicKey),
            100000000,
            5,
            publicKeyToHex(TOKEN_PROGRAM_ID),
            {
                accounts: [system_account, TOKEN_PROGRAM_ID],
                writableAccounts: [payer.publicKey, to_key_pair.publicKey],
                signers: [payer, to_key_pair],
            }
        );
    });

    it('create account with seed', async function create_account_with_seed() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction', 'TestingInstruction.abi');
        const base_keypair = Keypair.generate();
        const to_key_pair = await PublicKey.createWithSeed(base_keypair.publicKey, seed, TOKEN_PROGRAM_ID);

        await contract.functions.create_account_with_seed(
            publicKeyToHex(payer.publicKey),
            publicKeyToHex(to_key_pair),
            publicKeyToHex(base_keypair.publicKey),
            seed,
            100000000,
            5,
            publicKeyToHex(TOKEN_PROGRAM_ID),
            {
                accounts: [system_account, TOKEN_PROGRAM_ID],
                writableAccounts: [payer.publicKey, to_key_pair],
                signers: [payer, base_keypair]
            }
        );
    });

    it('assign', async function assign() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction', 'TestingInstruction.abi');
        const to_key_pair = Keypair.generate();

        const assign_account = new PublicKey('AddressLookupTab1e1111111111111111111111111');
        await contract.functions.assign(
            publicKeyToHex(to_key_pair.publicKey),
            publicKeyToHex(assign_account),
            {
                accounts: [system_account, payer.publicKey],
                writable_accounts: [to_key_pair.publicKey],
                signers: [to_key_pair],
            }
        )
    });

    it('assign with seed', async function assign_with_with_seed() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction', 'TestingInstruction.abi');

        const assign_account = new PublicKey('AddressLookupTab1e1111111111111111111111111');
        const to_key_pair = await PublicKey.createWithSeed(payer.publicKey, seed, assign_account);

        await contract.functions.assign_with_seed(
            publicKeyToHex(to_key_pair),
            publicKeyToHex(payer.publicKey),
            seed,
            publicKeyToHex(assign_account),
            {
                accounts: [system_account, assign_account],
                writableAccounts: [to_key_pair],
                signers: [payer]
            }
        );
    });

    it('transfer', async function transfer() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction', 'TestingInstruction.abi');
        const dest = new Keypair();

        await contract.functions.transfer(
            publicKeyToHex(payer.publicKey),
            publicKeyToHex(dest.publicKey),
            100000000,
            {
                accounts: [system_account],
                writableAccounts: [payer.publicKey, dest.publicKey],
                signers: [payer]
            }
        );
    });

    it('transfer with seed', async function transfer_with_seed() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction', 'TestingInstruction.abi');
        const dest = new Keypair();
        const assign_account = new PublicKey('AddressLookupTab1e1111111111111111111111111');
        const derived_payer = await PublicKey.createWithSeed(payer.publicKey, seed, assign_account);

        let signature = await connection.requestAirdrop(derived_payer, LAMPORTS_PER_SOL);
        await connection.confirmTransaction(signature, 'confirmed');

        await contract.functions.transfer_with_seed(
            publicKeyToHex(derived_payer),
            publicKeyToHex(payer.publicKey),
            seed,
            publicKeyToHex(assign_account),
            publicKeyToHex(dest.publicKey),
            100000000,
            {
                accounts: [system_account, assign_account],
                writableAccounts: [derived_payer, dest.publicKey],
                signers: [payer]
            }
        );
    });

    it('allocate', async function allocate() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction', 'TestingInstruction.abi');
        const account = Keypair.generate();

        await contract.functions.allocate(
            publicKeyToHex(account.publicKey),
            2,
            {
                accounts: [system_account],
                writableAccounts: [account.publicKey],
                signers: [account]
            }
        );
    });

    it('allocate with seed', async function allocate_with_seed() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction', 'TestingInstruction.abi');
        const account = Keypair.generate();
        const owner = new PublicKey('Stake11111111111111111111111111111111111111');
        const derived_key = await PublicKey.createWithSeed(account.publicKey, seed, owner);

        await contract.functions.allocate_with_seed(
            publicKeyToHex(derived_key),
            publicKeyToHex(account.publicKey),
            seed,
            200,
            publicKeyToHex(owner),
            {
                accounts: [system_account, owner],
                writableAccounts: [derived_key],
                signers: [account],
            }
        );
    });

    it('create nonce account with seed', async function create_nonce_account_with_seed() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction', 'TestingInstruction.abi');
        const base_address = Keypair.generate();
        const derived_account = await PublicKey.createWithSeed(base_address.publicKey, seed, system_account);
        const authority = Keypair.generate();

        await contract.functions.create_nonce_account_with_seed(
            publicKeyToHex(payer.publicKey),
            publicKeyToHex(derived_account),
            publicKeyToHex(base_address.publicKey),
            seed,
            publicKeyToHex(authority.publicKey),
            100000000,
            {
                accounts: [system_account, recent_block_hashes, rentAddress],
                writableAccounts: [payer.publicKey, derived_account],
                signers: [payer, base_address]
            }
        );
    });

    it('nonce accounts', async function nonce_accounts() {
        const { contract, connection, payer, program } = await loadContract('TestingInstruction', 'TestingInstruction.abi');
        const nonce = Keypair.generate();
        const authority = Keypair.generate();

        await contract.functions.create_nonce_account(
            publicKeyToHex(payer.publicKey),
            publicKeyToHex(nonce.publicKey),
            publicKeyToHex(authority.publicKey),
            100000000,
            {
                accounts: [system_account, recent_block_hashes, rentAddress],
                writableAccounts: [payer.publicKey, nonce.publicKey],
                signers: [payer, nonce],
            }
        );

        await contract.functions.advance_nonce_account(
            publicKeyToHex(nonce.publicKey),
            publicKeyToHex(authority.publicKey),
            {
                accounts: [system_account, recent_block_hashes],
                writableAccounts: [nonce.publicKey],
                signers: [authority],
            }
        );

        await contract.functions.withdraw_nonce_account(
            publicKeyToHex(nonce.publicKey),
            publicKeyToHex(authority.publicKey),
            publicKeyToHex(payer.publicKey),
            1000,
            {
                accounts: [system_account, recent_block_hashes, rentAddress],
                writableAccounts: [nonce.publicKey, payer.publicKey],
                signers: [authority],
            }
        );

        const new_authority = Keypair.generate();
        await contract.functions.authorize_nonce_account(
            publicKeyToHex(nonce.publicKey),
            publicKeyToHex(authority.publicKey),
            publicKeyToHex(new_authority.publicKey),
            {
                accounts: [system_account],
                writableAccounts: [nonce.publicKey],
                signers: [authority],
            }
        );
    });
});
