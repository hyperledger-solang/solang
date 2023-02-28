import expect from 'expect';
import { AnchorProvider, Program } from '@project-serum/anchor';
import {
    PublicKey, AccountMeta,
    Keypair, Signer,
    Connection,
    LAMPORTS_PER_SOL,
    BpfLoader, Transaction,
    BPF_LOADER_PROGRAM_ID, SystemProgram, sendAndConfirmTransaction,
} from '@solana/web3.js';
import * as fs from "fs";

async function newAccountWithLamports(connection: Connection): Promise<Keypair> {
    const account = Keypair.generate();
    let signature = await connection.requestAirdrop(account.publicKey, 16 * LAMPORTS_PER_SOL);
    await connection.confirmTransaction(signature, 'confirmed');
    return account;
}

describe('Call Anchor program from Solidity via IDL', () => {
    it('call_anchor', async function () {
        // This program instantiates an anchor program, calls various functions on it and checks the return values

        const connection = new Connection("http://localhost:8899", {
            commitment: "confirmed",
            confirmTransactionInitialTimeout: 1e6,
        });

        const payer = await newAccountWithLamports(connection);
        const callAnchorProgramId = Keypair.generate();
        await BpfLoader.load(connection, payer, callAnchorProgramId, fs.readFileSync("./tests/call_anchor.so"), BPF_LOADER_PROGRAM_ID);

        const file_name = "call_anchor";
        const idl = JSON.parse(fs.readFileSync("tests/" + file_name + ".json", 'utf8'));
        const storage = Keypair.generate();

        const provider = AnchorProvider.env();

        const data = Keypair.generate();

        await create_account(provider, storage, callAnchorProgramId.publicKey, 8192);

        const program = new Program(idl, callAnchorProgramId.publicKey, provider);

        // create account
        await program.methods.new(data.publicKey)
            .accounts({ dataAccount: storage.publicKey })
            .rpc();

        const ret = await program.methods.data().accounts({ dataAccount: storage.publicKey }).view();
        expect(ret).toEqual(data.publicKey);

        const remainingAccounts: AccountMeta[] = [{
            pubkey: new PublicKey("z7FbDfQDfucxJz5o8jrGLgvSbdoeSqX5VrxBb5TVjHq"),
            isSigner: false,
            isWritable: false,
        }, {
            pubkey: data.publicKey,
            isSigner: true,
            isWritable: true,
        }, {
            pubkey: payer.publicKey,
            isSigner: true,
            isWritable: true,
        }];

        await program.methods.test(payer.publicKey)
            .accounts({ dataAccount: storage.publicKey })
            .remainingAccounts(remainingAccounts)
            .signers([data, payer])
            .rpc();
    });
});

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