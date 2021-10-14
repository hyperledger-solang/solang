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
import crypto from 'crypto';
const Web3EthAbi = require('web3-eth-abi');

const default_url: string = "http://localhost:8899";
const return_data_prefix = 'Program return: ';
const log_data_prefix = 'Program data: ';

export async function establishConnection(): Promise<TestConnection> {
    let url = process.env.RPC_URL || default_url;
    let connection = new Connection(url, 'confirmed');
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

export async function createProgramAddress(program: PublicKey): Promise<any> {
    while (true) {
        let seed = crypto.randomBytes(7);
        let pda: any = undefined;

        await PublicKey.createProgramAddress([seed], program).then(v => { pda = v; }).catch(_ => { });

        if (pda) {
            return { address: pda, seed };
        }
    }
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
                commitment: 'confirmed',
                preflightCommitment: undefined,
            },
        );

        console.log('Contract storage account', account.publicKey.toBase58());

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
    constructor(private programId: PublicKey, public contractStorageAccount: Keypair, private abi: string) { }

    encode_seeds(seeds: any[]): Buffer {
        let seed_encoded = Buffer.alloc(1 + seeds.map(seed => seed.seed.length + 1).reduce((a, b) => a + b, 0));

        seed_encoded.writeUInt8(seeds.length);
        let offset = 1;

        seeds.forEach((v) => {
            let seed = v.seed;

            seed_encoded.writeUInt8(seed.length, offset);
            offset += 1;
            seed.copy(seed_encoded, offset);
            offset += seed.length;
        });

        return seed_encoded;
    }

    async call_constructor(test: TestConnection, contract: string, params: string[], seeds: any[] = []): Promise<void> {
        let abi: AbiItem | undefined = JSON.parse(this.abi).find((e: AbiItem) => e.type == "constructor");

        let inputs = abi?.inputs! || [];

        const input = Web3EthAbi.encodeParameters(inputs, params);

        let hash = utils.keccak256(Buffer.from(contract));

        const data = Buffer.concat([
            this.contractStorageAccount.publicKey.toBuffer(),
            test.payerAccount.publicKey.toBuffer(),
            Buffer.from('0000000000000000', 'hex'),
            Buffer.from(hash.substr(2, 8), 'hex'),
            this.encode_seeds(seeds),
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
                commitment: 'confirmed',
                preflightCommitment: undefined,
            },
        );
    }

    async call_function(test: TestConnection, name: string, params: any[], pubkeys: PublicKey[] = [], seeds: any[] = [], signers: Keypair[] = [], instr: TransactionInstruction[] = []): Promise<{ [key: string]: any }> {
        let abi: AbiItem = JSON.parse(this.abi).find((e: AbiItem) => e.name == name);

        const input: string = Web3EthAbi.encodeFunctionCall(abi, params);
        const data = Buffer.concat([
            this.contractStorageAccount.publicKey.toBuffer(),
            test.payerAccount.publicKey.toBuffer(),
            Buffer.from('0000000000000000', 'hex'),
            Buffer.from('00000000', 'hex'),
            this.encode_seeds(seeds),
            Buffer.from(input.replace('0x', ''), 'hex')
        ]);

        let debug = 'calling function ' + name + ' [' + params + ']';

        let keys = [];

        seeds.forEach((seed) => {
            keys.push({ pubkey: seed.address, isSigner: false, isWritable: true });
        });

        keys.push({ pubkey: this.contractStorageAccount.publicKey, isSigner: false, isWritable: true });
        keys.push({ pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false });
        keys.push({ pubkey: PublicKey.default, isSigner: false, isWritable: false });

        for (let i = 0; i < pubkeys.length; i++) {
            // make each 2nd key writable (will be account storage for contract)
            keys.push({ pubkey: pubkeys[i], isSigner: false, isWritable: (i & 1) == 1 });
        }

        const instruction = new TransactionInstruction({
            keys,
            programId: this.programId,
            data,
        });

        signers.unshift(test.payerAccount);

        let tx = new Transaction();

        instr.forEach(i => tx.add(i));
        tx.add(instruction);

        let signature = await sendAndConfirmTransaction(
            test.connection,
            tx,
            signers,
            {
                skipPreflight: false,
                commitment: 'confirmed',
                preflightCommitment: undefined,
            },
        );

        if (abi.outputs?.length) {
            const parsedTx = await test.connection.getParsedConfirmedTransaction(
                signature,
            );

            let encoded = Buffer.from([]);

            let seen = 0;

            for (let message of parsedTx!.meta?.logMessages!) {
                if (message.startsWith(return_data_prefix)) {
                    let [program_id, return_data] = message.slice(return_data_prefix.length).split(" ");
                    encoded = Buffer.from(return_data, 'base64')
                    seen += 1;
                }
            }

            if (seen == 0) {
                throw 'return data not set';
            }

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

    async call_function_expect_revert(test: TestConnection, name: string, params: any[], pubkeys: PublicKey[] = [], seeds: any[] = [], signers: Keypair[] = []): Promise<string> {
        let abi: AbiItem = JSON.parse(this.abi).find((e: AbiItem) => e.name == name);

        const input: string = Web3EthAbi.encodeFunctionCall(abi, params);
        const data = Buffer.concat([
            this.contractStorageAccount.publicKey.toBuffer(),
            test.payerAccount.publicKey.toBuffer(),
            Buffer.from('0000000000000000', 'hex'),
            Buffer.from('00000000', 'hex'),
            this.encode_seeds(seeds),
            Buffer.from(input.replace('0x', ''), 'hex')
        ]);

        let debug = 'calling function ' + name + ' [' + params + ']';

        let keys = [];

        seeds.forEach((seed) => {
            keys.push({ pubkey: seed.address, isSigner: false, isWritable: true });
        });

        keys.push({ pubkey: this.contractStorageAccount.publicKey, isSigner: false, isWritable: true });
        keys.push({ pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false });
        keys.push({ pubkey: PublicKey.default, isSigner: false, isWritable: false });

        for (let i = 0; i < pubkeys.length; i++) {
            // make each 2nd key writable (will be account storage for contract)
            keys.push({ pubkey: pubkeys[i], isSigner: false, isWritable: (i & 1) == 1 });
        }

        const instruction = new TransactionInstruction({
            keys,
            programId: this.programId,
            data,
        });

        signers.unshift(test.payerAccount);

        const { err, logs } = (await test.connection.simulateTransaction(new Transaction().add(instruction),
            signers)).value;

        if (!err) {
            throw 'error is not falsy';
        }

        let encoded;
        let seen = 0;

        for (let message of logs!) {
            if (message.startsWith(return_data_prefix)) {
                let [program_id, return_data] = message.slice(return_data_prefix.length).split(" ");
                encoded = Buffer.from(return_data, 'base64')
                seen += 1;
            }
        }

        if (seen == 0) {
            throw 'return data not set';
        }

        if (encoded?.readUInt32BE(0) != 0x08c379a0) {
            throw 'signature not correct';
        }

        return Web3EthAbi.decodeParameter('string', encoded.subarray(4).toString('hex'));
    }

    async call_function_events(test: TestConnection, name: string, params: any[], pubkeys: PublicKey[] = [], seeds: any[] = [], signers: Keypair[] = []): Promise<any[]> {
        let abi: AbiItem[] = JSON.parse(this.abi);
        let func = abi.find((e: AbiItem) => e.name == name)!;

        const input: string = Web3EthAbi.encodeFunctionCall(func, params);
        const data = Buffer.concat([
            this.contractStorageAccount.publicKey.toBuffer(),
            test.payerAccount.publicKey.toBuffer(),
            Buffer.from('0000000000000000', 'hex'),
            Buffer.from('00000000', 'hex'),
            this.encode_seeds(seeds),
            Buffer.from(input.replace('0x', ''), 'hex')
        ]);

        let debug = 'calling function ' + name + ' [' + params + ']';

        let keys = [];

        seeds.forEach((seed) => {
            keys.push({ pubkey: seed.address, isSigner: false, isWritable: true });
        });

        keys.push({ pubkey: this.contractStorageAccount.publicKey, isSigner: false, isWritable: true });
        keys.push({ pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false });
        keys.push({ pubkey: PublicKey.default, isSigner: false, isWritable: false });

        for (let i = 0; i < pubkeys.length; i++) {
            // make each 2nd key writable (will be account storage for contract)
            keys.push({ pubkey: pubkeys[i], isSigner: false, isWritable: (i & 1) == 1 });
        }

        const instruction = new TransactionInstruction({
            keys,
            programId: this.programId,
            data,
        });

        signers.unshift(test.payerAccount);

        const { err, logs } = (await test.connection.simulateTransaction(new Transaction().add(instruction),
            signers)).value;

        if (err) {
            throw 'error is not falsy';
        }

        let events = [];
        let event_abis = new Map();

        for (let index in abi) {
            let e = abi[index];

            if (e.type == "event") {
                let signature = Web3EthAbi.encodeEventSignature(e);
                event_abis.set(signature, e);
            }
        }

        for (let message of logs!) {
            if (message.startsWith(log_data_prefix)) {
                let fields = message.slice(log_data_prefix.length).split(" ");
                if (fields.length != 2) {
                    throw "expecting two fields for return data";
                }
                let topic_data = Buffer.from(fields[0], 'base64');
                let topics = [];

                for (let offset = 0; offset < topic_data.length; offset += 32) {
                    topics.push('0x' + topic_data.subarray(offset, offset + 32).toString('hex'));
                }

                let data = '0x' + Buffer.from(fields[1], 'base64').toString('hex');

                let event = event_abis.get(topics[0])!;

                events.push(Web3EthAbi.decodeLog(event.inputs, data, topics.slice(1)));
            }
        }

        return events;
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

    get_storage_keypair(): Keypair {
        return this.contractStorageAccount;
    }
}
