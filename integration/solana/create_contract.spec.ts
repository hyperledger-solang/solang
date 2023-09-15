// SPDX-License-Identifier: Apache-2.0

import { Connection, Keypair, PublicKey, sendAndConfirmTransaction, SystemProgram, Transaction } from '@solana/web3.js';
import expect from 'expect';
import { Program, Provider, BN } from '@coral-xyz/anchor';
import { create_account, loadContractAndCallConstructor } from './setup';
import fs from 'fs';

describe('ChildContract', function () {
    this.timeout(150000);

    let program: Program;
    let storage: Keypair
    let payer: Keypair;
    let provider: Provider;

    before(async function () {
        ({ program, storage, payer, provider } = await loadContractAndCallConstructor('creator'));
    });

    it('Create Contract', async function () {
        let child_program = new PublicKey("Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT");
        let child = Keypair.generate();

        const signature = await program.methods.createChild()
            .accounts({
                dataAccount: storage.publicKey,
                Child_programId: child_program,
                payer: payer.publicKey,
                Child_dataAccount: child.publicKey,
            })
            .signers([payer, child])
            .rpc({ commitment: 'confirmed' });

        const tx = await provider.connection.getTransaction(signature, { commitment: 'confirmed' });

        expect(tx?.meta?.logMessages!.toString()).toContain('In child constructor');
        expect(tx?.meta?.logMessages!.toString()).toContain('Hello there');

        const info = await provider.connection.getAccountInfo(child.publicKey);

        expect(info?.data.length).toEqual(518);
    });

    it('Creates Contract with seed1', async function () {
        let seed_program = new PublicKey("SeedHw4CsFsDEGu2AVwFM1toGXsbAJSKnb7kS8TrLxu");
        let seed = Buffer.from("chai");

        let [address, bump] = await PublicKey.findProgramAddress([seed], seed_program);

        const signature = await program.methods.createSeed1(
            seed, Buffer.from([bump]), new BN(711))
            .accounts({
                Seed1_programId: seed_program,
                payer: payer.publicKey,
                Seed1_dataAccount: address,
            })
            .signers([payer])
            .rpc({ commitment: 'confirmed' });

        const tx = await provider.connection.getTransaction(signature, { commitment: 'confirmed' });

        const logs = tx?.meta?.logMessages!;

        expect(logs.toString()).toContain('In Seed1 constructor');
        expect(logs.toString()).toContain('Hello from Seed1');

        const info = await provider.connection.getAccountInfo(address);

        expect(info?.data.length).toEqual(711);
    });

    it('Creates Contract with seed2', async function () {
        let seed_program = new PublicKey("Seed23VDZ9HFCfKvFwmemB6dpi25n5XjZdP52B2RUmh");
        let bare_seed = Buffer.from("poppy");

        let [address, bump] = await PublicKey.findProgramAddress([Buffer.from("sunflower"), bare_seed], seed_program);

        let seed = Buffer.concat([bare_seed, Buffer.from([bump])]);

        const signature = await program.methods.createSeed2(
            seed, new BN(9889))
            .accounts({
                Seed2_programId: seed_program,
                payer: payer.publicKey,
                Seed2_dataAccount: address,
            })
            .signers([payer])
            .rpc({ commitment: 'confirmed' });

        const tx = await provider.connection.getTransaction(signature, { commitment: 'confirmed' });

        const logs = tx?.meta?.logMessages!;

        expect(logs.toString()).toContain('In Seed2 constructor');

        const info = await provider.connection.getAccountInfo(address);

        expect(info?.data.length).toEqual(9889);

        const idl = JSON.parse(fs.readFileSync('Seed2.json', 'utf8'));

        const seed2 = new Program(idl, seed_program, provider);

        let res = await seed2.methods.check()
            .accounts({ dataAccount: address })
            .simulate();

        expect(res.raw.toString()).toContain('I am PDA.');
    });

    it('Create Contract with account metas vector', async function () {
        let child = Keypair.generate();
        let child_program = new PublicKey("Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT");

        const signature = await program.methods.createChildWithMetas(child.publicKey, payer.publicKey)
            .accounts({
                dataAccount: storage.publicKey,
                Child_programId: child_program,
            })
            .remainingAccounts([
                { pubkey: child.publicKey, isSigner: true, isWritable: true },
                { pubkey: payer.publicKey, isSigner: true, isWritable: true },
            ])
            .signers([payer, child])
            .rpc({ commitment: 'confirmed' });

        const tx = await provider.connection.getTransaction(signature, { commitment: 'confirmed' });

        expect(tx?.meta?.logMessages!.toString()).toContain('In child constructor');
        expect(tx?.meta?.logMessages!.toString()).toContain('I am using metas');

        const info = await provider.connection.getAccountInfo(child.publicKey);

        expect(info?.data.length).toEqual(518);
    });

    it('Create Contract without annotations', async function () {
        const child = Keypair.generate();
        const child_program = new PublicKey("8gTkAidfM82u3DGbKcZpHwL5p47KQA16MDb4WmrHdmF6");

        await create_account(child, child_program, 8192);
        const signature = await program.methods.createWithoutAnnotation()
            .accounts({
                MyCreature_dataAccount: child.publicKey,
                MyCreature_programId: child_program,
            })
            .rpc({ commitment: 'confirmed' });

        const tx = await provider.connection.getTransaction(signature, {
            commitment: 'confirmed',
            maxSupportedTransactionVersion: undefined,
        });

        expect(tx?.meta?.logMessages!.toString()).toContain('In child constructor');
        expect(tx?.meta?.logMessages!.toString()).toContain('say_my_name');
    });
});
