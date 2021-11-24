import { Connection, Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { Contract } from '@solana/solidity';
import fs from 'fs';

const endpoint: string = process.env.RPC_URL || "http://localhost:8899";
const PROGRAM_SO: Buffer = fs.readFileSync('bundle.so');

export async function loadContract(name: string, abifile: string, args: any[] = [], space: number = 8192):
    Promise<{ contract: Contract, connection: Connection, payer: Keypair, program: Keypair, storage: Keypair }> {
    const abi = JSON.parse(fs.readFileSync(abifile, 'utf8'));

    const connection = new Connection(endpoint, 'confirmed');

    const payerAccount = await newAccountWithLamports(connection, 10000000000);
    const program = Keypair.generate();
    const storage = Keypair.generate();
    const contract = new Contract(connection, program.publicKey, storage.publicKey, abi, payerAccount);

    await contract.load(program, PROGRAM_SO);

    await contract.deploy(name, args, program, storage, space);

    return { contract, connection, payer: payerAccount, program, storage };
}

export async function load2ndContract(connection: Connection, program: Keypair, payerAccount: Keypair, name: string, abifile: string, args: any[] = [], space: number = 8192): Promise<Contract> {
    const abi = JSON.parse(fs.readFileSync(abifile, 'utf8'));

    const storage = Keypair.generate();
    const contract = new Contract(connection, program.publicKey, storage.publicKey, abi, payerAccount);

    await contract.deploy(name, args, program, storage, space);

    return contract;
}

async function newAccountWithLamports(
    connection: Connection,
    lamports: number = LAMPORTS_PER_SOL
): Promise<Keypair> {
    const account = Keypair.generate();

    console.log('Airdropping SOL to a new wallet ...');
    const signature = await connection.requestAirdrop(account.publicKey, lamports);
    await connection.confirmTransaction(signature, 'confirmed');

    return account;
}