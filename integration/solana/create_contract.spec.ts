// SPDX-License-Identifier: Apache-2.0

import { Connection, Keypair, PublicKey, sendAndConfirmTransaction, SystemProgram, Transaction } from '@solana/web3.js';
import expect from 'expect';
import { Contract } from '@solana/solidity';
import { loadContract } from './setup';
import fs from 'fs';

describe('ChildContract', function () {
    this.timeout(150000);

    let contract: Contract;
    let payer: Keypair;
    let connection: Connection;

    before(async function () {
        ({ contract, payer, connection } = await loadContract('creator'));
    });

    it('Create Contract', async function () {
        let child_program = new PublicKey("Chi1d5XD6nTAp2EyaNGqMxZzUjh6NvhXRxbGHP3D1RaT");
        let child = Keypair.generate();

        const { logs } = await contract.functions.create_child(child.publicKey.toBytes(), payer.publicKey.toBytes(), {
            accounts: [child_program],
            writableAccounts: [child.publicKey],
            signers: [child, payer],
        });

        expect(logs.toString()).toContain('In child constructor');
        expect(logs.toString()).toContain('Hello there');

        const info = await contract.connection.getAccountInfo(child.publicKey);

        expect(info?.data.length).toEqual(518);
    });

    it('Creates Contract with seed1', async function () {
        let seed_program = new PublicKey("SeedHw4CsFsDEGu2AVwFM1toGXsbAJSKnb7kS8TrLxu");
        let seed: Uint8Array = Buffer.from("chai");

        let [address, bump] = await PublicKey.findProgramAddress([seed], seed_program);

        const { logs } = await contract.functions.create_seed1(
            address.toBytes(), payer.publicKey.toBytes(), seed, Buffer.from([bump]), 711, {
            accounts: [seed_program],
            writableAccounts: [address],
            signers: [payer],
        });

        expect(logs.toString()).toContain('In Seed1 constructor');
        expect(logs.toString()).toContain('Hello from Seed1');

        const info = await contract.connection.getAccountInfo(address);

        expect(info?.data.length).toEqual(711);
    });

    it('Creates Contract with seed2', async function () {
        let seed_program = new PublicKey("Seed23VDZ9HFCfKvFwmemB6dpi25n5XjZdP52B2RUmh");
        let bare_seed = Buffer.from("poppy");

        let [address, bump] = await PublicKey.findProgramAddress([Buffer.from("sunflower"), bare_seed], seed_program);

        let seed = Buffer.concat([bare_seed, Buffer.from([bump])]);

        const { logs } = await contract.functions.create_seed2(
            address.toBytes(), payer.publicKey.toBytes(), seed, 9889, {
            accounts: [seed_program],
            writableAccounts: [address],
            signers: [payer],
        });

        expect(logs.toString()).toContain('In Seed2 constructor');

        const info = await contract.connection.getAccountInfo(address);

        expect(info?.data.length).toEqual(9889 + 23);

        const abi = JSON.parse(fs.readFileSync('Seed2.abi', 'utf8'));

        let seed2 = new Contract(connection, seed_program, address, abi, payer);

        let res = await seed2.functions.check();

        expect(res.logs.toString()).toContain('I am PDA.');
    });
});
