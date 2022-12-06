// SPDX-License-Identifier: Apache-2.0

import { Connection, Keypair, LAMPORTS_PER_SOL, BpfLoader, BPF_LOADER_PROGRAM_ID } from '@solana/web3.js';
import { Contract } from '@solana/solidity';
import fs from 'fs';

const endpoint: string = process.env.RPC_URL || "http://127.0.0.1:8899";

export async function loadContract(name: string, args: any[] = [], space: number = 8192):
    Promise<{ contract: Contract, connection: Connection, payer: Keypair, program: Keypair, storage: Keypair }> {

    const abi = JSON.parse(fs.readFileSync(`${name}.abi`, 'utf8'));

    const connection = new Connection(endpoint, 'confirmed');

    const payerAccount = load_key('payer.key');
    const program = load_key(`${name}.key`);

    const storage = Keypair.generate();
    const contract = new Contract(connection, program.publicKey, storage.publicKey, abi, payerAccount);

    await contract.deploy(name, args, storage, space);

    return { contract, connection, payer: payerAccount, program, storage };
}

export function newConnectionAndAccounts(name: string): [Connection, Keypair, Keypair] {
    const connection = new Connection(endpoint, 'confirmed');
    const payerAccount = load_key('payer.key');
    const program = load_key(`${name}.key`);
    return [connection, payerAccount, program];
}

export async function loadContractWithExistingConnectionAndPayer(connection: Connection, payerAccount: Keypair, name: string, args: any[] = [], space: number = 8192): Promise<Contract> {
    const abi = JSON.parse(fs.readFileSync(`${name}.abi`, 'utf8'));

    const storage = Keypair.generate();
    const program = load_key(`${name}.key`);
    const contract = new Contract(connection, program.publicKey, storage.publicKey, abi, payerAccount);

    await contract.deploy(name, args, storage, space);

    return contract;
}

function load_key(filename: string): Keypair {
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

    const connection = new Connection(endpoint, {
        commitment: "confirmed",
        confirmTransactionInitialTimeout: 100000,
    });
    const payer = await newAccountWithLamports(connection);

    write_key('payer.key', payer);

    let files = fs.readdirSync(__dirname);
    for (let index in files) {
        let file = files[index];

        if (file.endsWith('.so')) {
            let name = file.slice(0, -3);
            let program;

            if (fs.existsSync(`${name}.key`)) {
                program = load_key(`${name}.key`);
            } else {
                program = Keypair.generate();
            }

            console.log(`Loading ${name} at ${program.publicKey}...`);
            const program_so = fs.readFileSync(file);
            await BpfLoader.load(connection, payer, program, program_so, BPF_LOADER_PROGRAM_ID);
            console.log(`Done loading ${name} ...`);

            write_key(`${name}.key`, program);
        }
    }
}

if (require.main === module) {
    (async () => {
        await setup();
    })();
}
