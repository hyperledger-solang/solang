// SPDX-License-Identifier: Apache-2.0

import { Connection, Keypair, LAMPORTS_PER_SOL, PublicKey, BpfLoader, Transaction, SystemProgram, BPF_LOADER_PROGRAM_ID, TransactionExpiredBlockheightExceededError } from '@solana/web3.js';
import { AnchorProvider, Program } from '@project-serum/anchor';
import fs from 'fs';

const endpoint: string = process.env.RPC_URL || "http://127.0.0.1:8899";

export async function loadContract(name: string, args: any[] = [], space: number = 8192):
    Promise<{ program: Program, payer: Keypair, provider: AnchorProvider, storage: Keypair, program_key: PublicKey }> {

    const idl = JSON.parse(fs.readFileSync(`${name}.json`, 'utf8'));

    const payer = loadKey('payer.key');

    process.env['ANCHOR_WALLET'] = 'payer.key';

    const provider = AnchorProvider.local(endpoint);

    const storage = Keypair.generate();

    const program_key = loadKey(`${name}.key`);

    await create_account(provider, storage, program_key.publicKey, space);

    const program = new Program(idl, program_key.publicKey, provider);

    await program.methods.new(...args)
        .accounts({ dataAccount: storage.publicKey })
        .rpc();

    return { provider, program, payer, storage, program_key: program_key.publicKey };
}

async function create_account(provider: AnchorProvider, account: Keypair, programId: PublicKey, space: number) {
    const lamports = await provider.connection.getMinimumBalanceForRentExemption(space);

    const transaction = new Transaction();

    transaction.add(
        SystemProgram.createAccount({
            fromPubkey: provider.wallet.publicKey,
            newAccountPubkey: account.publicKey,
            lamports,
            space,
            programId,
        }));

    await provider.sendAndConfirm(transaction, [account]);
}

export function newConnectionAndPayer(): [Connection, Keypair] {
    const connection = newConnection();
    const payerAccount = loadKey('payer.key');
    return [connection, payerAccount];
}

export async function loadContractWithProvider(provider: AnchorProvider, name: string, args: any[] = [], space: number = 8192):
    Promise<{ program: Program, storage: Keypair, program_key: PublicKey }> {

    const idl = JSON.parse(fs.readFileSync(`${name}.json`, 'utf8'));

    const storage = Keypair.generate();
    const program_key = loadKey(`${name}.key`);

    await create_account(provider, storage, program_key.publicKey, space);

    const program = new Program(idl, program_key.publicKey, provider);

    await program.methods.new(...args)
        .accounts({ dataAccount: storage.publicKey })
        .rpc();

    return { program, storage, program_key: program_key.publicKey };
}

function loadKey(filename: string): Keypair {
    const contents = fs.readFileSync(filename).toString();
    const bs = Uint8Array.from(JSON.parse(contents));

    return Keypair.fromSecretKey(bs);
}

async function newAccountWithLamports(connection: Connection): Promise<Keypair> {
    const account = Keypair.generate();

    console.log('Airdropping SOL to a new wallet ...');
    let signature = await connection.requestAirdrop(account.publicKey, 100 * LAMPORTS_PER_SOL);
    await connection.confirmTransaction(signature, 'confirmed');

    return account;
}

async function setup() {
    let write_key = (file_name: string, key: Keypair) => {
        fs.writeFileSync(file_name, JSON.stringify(Array.from(key.secretKey)));
    };

    let connection = newConnection();
    const payer = await newAccountWithLamports(connection);

    write_key('payer.key', payer);

    let files = fs.readdirSync(__dirname);
    for (let index in files) {
        let file = files[index];

        if (file.endsWith('.so')) {
            let name = file.slice(0, -3);
            let program;

            if (fs.existsSync(`${name}.key`)) {
                program = loadKey(`${name}.key`);
            } else {
                program = Keypair.generate();
            }

            console.log(`Loading ${name} at ${program.publicKey}...`);
            const program_so = fs.readFileSync(file);
            for (; ;) {
                try {
                    await BpfLoader.load(connection, payer, program, program_so, BPF_LOADER_PROGRAM_ID);
                    break;
                } catch (e) {
                    if (e instanceof TransactionExpiredBlockheightExceededError) {
                        console.log(e);
                        console.log('retrying...');
                        connection = newConnection();
                    } else {
                        throw e;
                    }
                }
            }
            console.log(`Done loading ${name} ...`);

            write_key(`${name}.key`, program);
        }
    }

    // If there was a TransactionExpiredBlockheightExceededError exception, then
    // setup.ts does not exit. I have no idea why
    process.exit();
}

function newConnection(): Connection {
    const connection = new Connection(endpoint, {
        commitment: "confirmed",
        confirmTransactionInitialTimeout: 1e6,
    });

    return connection;
}

if (require.main === module) {
    (async () => {
        await setup();
    })();
}
