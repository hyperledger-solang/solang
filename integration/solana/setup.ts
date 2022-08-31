import { Connection, Keypair, LAMPORTS_PER_SOL, BpfLoader, BPF_LOADER_PROGRAM_ID } from '@solana/web3.js';
import { Contract } from '@solana/solidity';
import fs from 'fs';

const endpoint: string = process.env.RPC_URL || "http://localhost:8899";
const PROGRAM_SO: Buffer = fs.readFileSync('bundle.so');

export async function loadContract(name: string, abifile: string, args: any[] = [], space: number = 8192):
    Promise<{ contract: Contract, connection: Connection, payer: Keypair, program: Keypair, storage: Keypair }> {

    const abi = JSON.parse(fs.readFileSync(abifile, 'utf8'));

    const connection = new Connection(endpoint, 'confirmed');

    const payerAccount = load_key('payer.key');
    const program = load_key('program.key');
    const storage = Keypair.generate();
    const contract = new Contract(connection, program.publicKey, storage.publicKey, abi, payerAccount);

    await contract.deploy(name, args, storage, space);

    return { contract, connection, payer: payerAccount, program, storage };
}

export async function load2ndContract(connection: Connection, program: Keypair, payerAccount: Keypair, name: string, abifile: string, args: any[] = [], space: number = 8192): Promise<Contract> {
    const abi = JSON.parse(fs.readFileSync(abifile, 'utf8'));

    const storage = Keypair.generate();
    const contract = new Contract(connection, program.publicKey, storage.publicKey, abi, payerAccount);

    await contract.deploy(name, args, storage, space);

    return contract;
}

function load_key(filename: string): Keypair {
    const contents = fs.readFileSync(filename).toString();
    const bs = Uint8Array.from(contents.split(',').map(v => Number(v)));

    return Keypair.fromSecretKey(bs);
}

async function newAccountWithLamports(connection: Connection): Promise<Keypair> {
    const account = Keypair.generate();

    console.log('Airdropping SOL to a new wallet ...');
    let signature = await connection.requestAirdrop(account.publicKey, LAMPORTS_PER_SOL);
    await connection.confirmTransaction(signature, 'confirmed');
    signature = await connection.requestAirdrop(account.publicKey, 5*LAMPORTS_PER_SOL);
    await connection.confirmTransaction(signature, 'confirmed');
    signature = await connection.requestAirdrop(account.publicKey, 5*LAMPORTS_PER_SOL);
    await connection.confirmTransaction(signature, 'confirmed');
    signature = await connection.requestAirdrop(account.publicKey, 5*LAMPORTS_PER_SOL);
    await connection.confirmTransaction(signature, 'confirmed');
    signature = await connection.requestAirdrop(account.publicKey, 5*LAMPORTS_PER_SOL);
    await connection.confirmTransaction(signature, 'confirmed');

    return account;
}


async function setup() {

    const connection = new Connection(endpoint, {
        commitment: "confirmed",
        confirmTransactionInitialTimeout: 100000,
    });
    const payer = await newAccountWithLamports(connection);

    const program = Keypair.generate();

    console.log('Loading bundle.so ...');
    await BpfLoader.load(connection, payer, program, PROGRAM_SO, BPF_LOADER_PROGRAM_ID);
    console.log('Done loading bundle.so ...');

    fs.writeFileSync('payer.key', String(payer.secretKey));
    fs.writeFileSync('program.key', String(program.secretKey));
}

if (require.main === module) {
    (async () => {
        await setup();
    })();
}
