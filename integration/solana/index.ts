import {
    Account,
    Connection,
    BpfLoader,
    BPF_LOADER_PROGRAM_ID,
    PublicKey,
    LAMPORTS_PER_SOL,
    SystemProgram,
    TransactionInstruction,
    Transaction,
    sendAndConfirmTransaction,
} from '@solana/web3.js';
import fs from 'fs';
import { AbiItem } from 'web3-utils';
const Web3EthAbi = require('web3-eth-abi');

const default_url: string = "http://localhost:8899";

export async function establishConnection(): Promise<TestConnection> {
    let url = process.env.RPC_URL || default_url;
    let connection = new Connection(url, 'recent');
    const version = await connection.getVersion();
    console.log('Connection to cluster established:', url, version);

    // Fund a new payer via airdrop
    let payerAccount = await newAccountWithLamports(connection);

    const lamports = await connection.getBalance(payerAccount.publicKey);
    console.log(
        'Using account',
        payerAccount.publicKey.toBase58(),
        'containing',
        lamports / LAMPORTS_PER_SOL,
        'Sol to pay for fees',
    );

    return new TestConnection(connection, payerAccount);
}

const sleep = (ms: number) => new Promise(res => setTimeout(res, ms));

async function newAccountWithLamports(
    connection: Connection,
    lamports: number = 100000000,
): Promise<Account> {
    const account = new Account();

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
        console.log('Airdrop retry ' + retries);
    }
    throw new Error(`Airdrop of ${lamports} failed`);
}

class TestConnection {
    constructor(public connection: Connection, public payerAccount: Account) { }

    async createStorageAccount(programId: PublicKey, space: number): Promise<Account> {
        const lamports = await this.connection.getMinimumBalanceForRentExemption(
            space
        );

        let account = new Account();

        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: this.payerAccount.publicKey,
                newAccountPubkey: account.publicKey,
                lamports,
                space,
                programId,
            }),
        );

        await sendAndConfirmTransaction(
            this.connection,
            transaction,
            [this.payerAccount, account],
            {
                skipPreflight: false,
                commitment: 'recent',
                preflightCommitment: undefined,
            },
        );

        return account;
    }

    async loadProgram(sopath: string, abipath: string): Promise<Program> {
        console.log(`Loading ${sopath} ...`)

        const data: Buffer = fs.readFileSync(sopath);

        const abi: string = fs.readFileSync(abipath, 'utf-8');

        const programAccount = new Account();

        await BpfLoader.load(
            this.connection,
            this.payerAccount,
            programAccount,
            data,
            BPF_LOADER_PROGRAM_ID,
        );
        const programId = programAccount.publicKey;

        console.log('Program loaded to account', programId.toBase58());

        const returnDataAccount = await this.createStorageAccount(programId, 100);
        const contractStorageAccount = await this.createStorageAccount(programId, 10);

        return new Program(programId, returnDataAccount, contractStorageAccount, abi);
    }
}

class Program {
    constructor(private programId: PublicKey, private returnDataAccount: Account, private contractStorageAccount: Account, private abi: string) { }

    async call_constructor(test: TestConnection, params: string[]): Promise<void> {
        let abi: AbiItem = JSON.parse(this.abi).find((e: AbiItem) => e.type == "constructor");

        const input = Web3EthAbi.encodeParameters(abi.inputs!, params);

        console.log('calling constructor ' + params);

        const instruction = new TransactionInstruction({
            keys: [
                { pubkey: this.returnDataAccount.publicKey, isSigner: false, isWritable: true },
                { pubkey: this.contractStorageAccount.publicKey, isSigner: false, isWritable: true }],
            programId: this.programId,
            data: Buffer.from(input.substring(2), 'hex'),
        });

        await sendAndConfirmTransaction(
            test.connection,
            new Transaction().add(instruction),
            [test.payerAccount],
            {
                skipPreflight: false,
                commitment: 'recent',
                preflightCommitment: undefined,
            },
        );
    }

    async call_function(test: TestConnection, name: string, params: string[]): Promise<{ [key: string]: any }> {
        let abi: AbiItem = JSON.parse(this.abi).find((e: AbiItem) => e.name == name);

        const input: string = Web3EthAbi.encodeFunctionCall(abi, params);

        let debug = 'calling function ' + name + params;

        const instruction = new TransactionInstruction({
            keys: [
                { pubkey: this.returnDataAccount.publicKey, isSigner: false, isWritable: true },
                { pubkey: this.contractStorageAccount.publicKey, isSigner: false, isWritable: true }],
            programId: this.programId,
            data: Buffer.from(input.substr(2), 'hex'),
        });

        await sendAndConfirmTransaction(
            test.connection,
            new Transaction().add(instruction),
            [test.payerAccount],
            {
                skipPreflight: false,
                commitment: 'recent',
                preflightCommitment: undefined,
            },
        );

        if (abi.outputs?.length) {
            const accountInfo = await test.connection.getAccountInfo(this.returnDataAccount.publicKey);

            let length = Number(accountInfo!.data.readBigInt64LE(0));

            let encoded = accountInfo!.data.slice(8, length + 8);

            let returns = Web3EthAbi.decodeParameters(abi.outputs!, encoded.toString('hex'));

            debug += " returns [";
            for (let i = 0; i.toString() in returns; i++) {
                debug += returns[i];
            }
            debug += "]"
            console.log(debug);

            return returns;
        } else {
            console.log(debug);
            return [];
        }
    }

    async contract_storage(test: TestConnection, upto: number): Promise<Buffer> {
        const accountInfo = await test.connection.getAccountInfo(this.contractStorageAccount.publicKey);

        return accountInfo!.data;
    }

    async return_data(test: TestConnection, upto: number): Promise<Buffer> {
        const accountInfo = await test.connection.getAccountInfo(this.returnDataAccount.publicKey);

        return accountInfo!.data;
    }
}
