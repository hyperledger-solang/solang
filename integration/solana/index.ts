import {
    Keypair,
    Connection,
    BpfLoader,
    BPF_LOADER_PROGRAM_ID,
    PublicKey,
    LAMPORTS_PER_SOL,
    SystemProgram,
    TransactionInstruction,
    Transaction,
    sendAndConfirmTransaction, SYSVAR_CLOCK_PUBKEY,
} from '@solana/web3.js';
import fs from 'fs';
import { AbiItem } from 'web3-utils';
import { utils } from 'ethers';
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
    lamports: number = 10000000000,
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
        console.log('Airdrop retry ' + retries);
    }
    throw new Error(`Airdrop of ${lamports} failed`);
}

class TestConnection {
    constructor(public connection: Connection, public payerAccount: Keypair) { }

    async createStorageAccount(programId: PublicKey, space: number): Promise<Keypair> {
        const lamports = await this.connection.getMinimumBalanceForRentExemption(
            space
        );

        let account = Keypair.generate();

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

    async loadProgram(sopath: string, abipath: string, contractStorageSize: number = 2048): Promise<Program> {
        console.log(`Loading ${sopath} ...`)

        const data: Buffer = fs.readFileSync(sopath);

        const abi: string = fs.readFileSync(abipath, 'utf-8');

        const programAccount = Keypair.generate();

        await BpfLoader.load(
            this.connection,
            this.payerAccount,
            programAccount,
            data,
            BPF_LOADER_PROGRAM_ID,
        );
        const programId = programAccount.publicKey;

        console.log('Program loaded to account', programId.toBase58());

        const contractStorageAccount = await this.createStorageAccount(programId, contractStorageSize);

        return new Program(programId, contractStorageAccount, abi);
    }
}

class Program {
    constructor(private programId: PublicKey, private contractStorageAccount: Keypair, private abi: string) { }

    async call_constructor(test: TestConnection, contract: string, params: string[]): Promise<void> {
        let abi: AbiItem | undefined = JSON.parse(this.abi).find((e: AbiItem) => e.type == "constructor");

        let inputs = abi?.inputs! || [];

        const input = Web3EthAbi.encodeParameters(inputs, params);

        let hash = utils.keccak256(Buffer.from(contract));

        const data = Buffer.concat([
            this.contractStorageAccount.publicKey.toBuffer(),
            Buffer.from(hash.substr(2, 8), 'hex'),
            Buffer.from(input.replace('0x', ''), 'hex')
        ]);

        console.log('calling constructor [' + params + ']');

        const instruction = new TransactionInstruction({
            keys: [
                { pubkey: this.contractStorageAccount.publicKey, isSigner: false, isWritable: true }],
            programId: this.programId,
            data,
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

    async call_function(test: TestConnection, name: string, params: any[], pubkeys: PublicKey[] = []): Promise<{ [key: string]: any }> {
        let abi: AbiItem = JSON.parse(this.abi).find((e: AbiItem) => e.name == name);

        const input: string = Web3EthAbi.encodeFunctionCall(abi, params);
        const data = Buffer.concat([
            this.contractStorageAccount.publicKey.toBuffer(),
            Buffer.from('00000000', 'hex'),
            Buffer.from(input.replace('0x', ''), 'hex')
        ]);

        let debug = 'calling function ' + name + ' [' + params + ']';

        let keys = [
            { pubkey: this.contractStorageAccount.publicKey, isSigner: false, isWritable: true },
            { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
        ];

        for (let i = 0; i < pubkeys.length; i++) {
            keys.push({ pubkey: pubkeys[i], isSigner: false, isWritable: true });
        }

        const instruction = new TransactionInstruction({
            keys,
            programId: this.programId,
            data,
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
            const accountInfo = await test.connection.getAccountInfo(this.contractStorageAccount.publicKey);

            let length = Number(accountInfo!.data.readUInt32LE(4));
            let offset = Number(accountInfo!.data.readUInt32LE(8));

            let encoded = accountInfo!.data.slice(offset, length + offset);

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

    all_keys(): PublicKey[] {
        return [this.programId, this.contractStorageAccount.publicKey];
    }

    get_program_key(): PublicKey {
        return this.programId;
    }

    get_storage_key(): PublicKey {
        return this.contractStorageAccount.publicKey;
    }
}
