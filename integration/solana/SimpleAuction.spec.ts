import { publicKeyToHex } from '@solana/solidity';
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

    await contract.deploy(name, args, program, storage, space);

    return { contract, connection, payer: payerAccount, program, storage };
}

async function setup() {
    const connection = new Connection(endpoint, 'confirmed');
    const payer = await newAccountWithLamports(connection);

    const program = Keypair.generate();

    console.log('Loading bundle.so ...');
    await BpfLoader.load(connection, payer, program, PROGRAM_SO, BPF_LOADER_PROGRAM_ID);
    console.log(`program loaded at ${program.publicKey}`);

    fs.writeFileSync('payer.key', String(payer.secretKey));
    fs.writeFileSync('program.key', String(program.secretKey));
}

export function delay(ms: number) {
    return new Promise(resolve => setTimeout(resolve, ms));
}


describe('Deploy solang contract and test', function () {
    this.timeout(500000);

    it('SimpleAution', async function () {
        let beneficiary = Keypair.generate();

        let { contract, connection, storage } = await loadContract('SimpleAuction', 'SimpleAuction.abi', [60, publicKeyToHex(beneficiary.publicKey)]);

        // ensure the beneficiary exists
        let signature = await connection.requestAirdrop(beneficiary.publicKey, LAMPORTS_PER_SOL);
        await connection.confirmTransaction(signature, 'confirmed');

        let bidder1 = await newAccountWithLamports(connection);
        let bidder2 = await newAccountWithLamports(connection);

        await contract.functions.bid({
            sender: bidder1.publicKey,
            signers: [bidder1],
            value: 102,
        });

        await contract.functions.bid({
            sender: bidder2.publicKey,
            signers: [bidder2],
            value: LAMPORTS_PER_SOL,
        });

        await delay(60 * 1000);

        await contract.functions.auctionEnd({
            signers: [storage],
            writableAccounts: [beneficiary.publicKey],
        });
    });
});

function load_key(filename: string): Keypair {
    const contents = fs.readFileSync(filename).toString();
    const bs = Uint8Array.from(contents.split(',').map(v => Number(v)));

    return Keypair.fromSecretKey(bs);
}

export async function newAccountWithLamports(connection: Connection): Promise<Keypair> {
    const account = Keypair.generate();

    console.log('Airdropping SOL to a new wallet ...');
    let signature = await connection.requestAirdrop(account.publicKey, LAMPORTS_PER_SOL);
    await connection.confirmTransaction(signature, 'confirmed');
    signature = await connection.requestAirdrop(account.publicKey, LAMPORTS_PER_SOL);
    await connection.confirmTransaction(signature, 'confirmed');
    signature = await connection.requestAirdrop(account.publicKey, LAMPORTS_PER_SOL);
    await connection.confirmTransaction(signature, 'confirmed');

    return account;
}


