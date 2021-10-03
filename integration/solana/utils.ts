
import { Connection, Keypair } from '@solana/web3.js';
import { Contract, Program } from '@solana/solidity';
import fs from 'fs';


const endpoint: string = process.env.RPC_URL || "http://localhost:8899";
const PROGRAM_SO: Buffer = fs.readFileSync('bundle.so');

export async function loadContract(name: string, abifile: string, args: any[] = [], space: number = 8192): Promise<[Contract, Connection, Keypair]> {
    const CONTRACT_ABI: string = fs.readFileSync(abifile, 'utf8');

    const connection = new Connection(endpoint, 'confirmed');

    const payerAccount = await newAccountWithLamports(connection, 10000000000);
    const program = await Program.load(connection, payerAccount, Keypair.generate(), PROGRAM_SO);

    const storageKeyPair = Keypair.generate();
    const deployRes = await program.deployContract({
        name,
        abi: CONTRACT_ABI,
        storageKeyPair,
        constructorArgs: args,
        space
    });

    return [deployRes.contract, connection, payerAccount];
}

export async function loadProgram(): Promise<[Program, Connection, Keypair]> {
    const connection = new Connection(endpoint, 'confirmed');

    const payerAccount = await newAccountWithLamports(connection, 10000000000);
    const program = await Program.load(connection, payerAccount, Keypair.generate(), PROGRAM_SO);

    return [program, connection, payerAccount];
}

async function newAccountWithLamports(
    connection: Connection,
    lamports: number = 10000000000
): Promise<Keypair> {
    const account = Keypair.generate();

    let retries = 10;
    await connection.requestAirdrop(account.publicKey, lamports);
    for (; ;) {
        await sleep(500);
        if (lamports == (await connection.getBalance(account.publicKey))) {
            return account;
        }
        if (--retries <= 0) {
            break;
        }
        // console.log('airdrop retry ' + retries);
    }
    throw new Error(`airdrop of ${lamports} failed`);
}

function sleep(ms: number) {
    return new Promise(function (resolve) {
        setTimeout(resolve, ms);
    });
}
