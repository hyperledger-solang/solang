import expect from 'expect';
import {Contract, publicKeyToHex} from '@solana/solidity';
import {
    SystemProgram,
    PublicKey,
    Keypair,
    Connection,
    LAMPORTS_PER_SOL,
    BpfLoader,
    BPF_LOADER_PROGRAM_ID
} from '@solana/web3.js';
import * as fs from "fs";

async function newAccountWithLamports(connection: Connection): Promise<Keypair> {
    const account = Keypair.generate();
    let signature = await connection.requestAirdrop(account.publicKey, 16*LAMPORTS_PER_SOL);
    await connection.confirmTransaction(signature, 'confirmed');
    return account;
}

describe('Call Anchor program from Solidity via IDL',  () => {

    it('call_anchor', async function () {
        // This program instantiates an anchor program, calls various functions on it and checks the return values

        const connection = new Connection("http://localhost:8899", {
            commitment: "confirmed",
            confirmTransactionInitialTimeout: 100000,
        });

        const payer = await newAccountWithLamports(connection);
        const program = Keypair.generate();
        await BpfLoader.load(connection, payer, program, fs.readFileSync("./tests/bundle.so"), BPF_LOADER_PROGRAM_ID);

        const file_name = "call_anchor";
        const abi = JSON.parse(fs.readFileSync("tests/" + file_name + ".abi", 'utf8'));
        const storage = Keypair.generate();
        const contract = new Contract(connection, program.publicKey, storage.publicKey, abi, payer);

        const data = Keypair.generate();
        await contract.deploy(file_name, [data.publicKey.toBytes()], storage, 8192);

        const programId = new PublicKey("z7FbDfQDfucxJz5o8jrGLgvSbdoeSqX5VrxBb5TVjHq");

        let { result } = await contract.functions.test(payer.publicKey.toBytes(), { accounts: [programId, SystemProgram.programId], signers: [data, payer] });

        expect(Number(result)).toStrictEqual(11);
    });
});
