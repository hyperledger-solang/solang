// SPDX-License-Identifier: Apache-2.0

import expect from 'expect';
import { createConnection, deploy, aliceKeypair, query, debug_buffer, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';

enum CallFlags {
    FORWARD_INPUT, CLONE_INPUT, TAIL_CALL, ALLOW_REENTRY
}

describe('Deploy the CallFlags contract and tests for various call flag combinations', () => {
    let conn: ApiPromise;
    let contract: ContractPromise;
    let alice: KeyringPair;
    const voyager = 987654321;
    const foo = [0, 0, 0, 0];

    before(async function () {
        alice = aliceKeypair();
        conn = await createConnection();
        const deployment = await deploy(conn, alice, 'CallFlags.contract', 0n);
        contract = new ContractPromise(conn, deployment.abi, deployment.address);
    });

    after(async function () {
        await conn.disconnect();
    });

    it('works with the reentry flag', async function () {
        const flags = [CallFlags.ALLOW_REENTRY];
        const answer = await query(conn, alice, contract, "echo", [contract.address, foo, voyager, flags]);
        expect(answer.output?.toJSON()).toStrictEqual(voyager);
    });

    it('works with the reentry and tail call flags', async function () {
        const flags = [CallFlags.ALLOW_REENTRY, CallFlags.TAIL_CALL];
        const answer = await query(conn, alice, contract, "echo", [contract.address, foo, voyager, flags]);
        expect(answer.output?.toJSON()).toStrictEqual(voyager);
    });

    it('works with the reentry and clone input flags', async function () {
        const flags = [CallFlags.ALLOW_REENTRY, CallFlags.CLONE_INPUT];
        const answer = await query(conn, alice, contract, "echo", [contract.address, foo, voyager, flags]);
        expect(answer.output?.toJSON()).toStrictEqual(voyager);
    });

    it('works with the reentry, tail call and clone input flags', async function () {
        const flags = [CallFlags.ALLOW_REENTRY, CallFlags.TAIL_CALL, CallFlags.CLONE_INPUT];
        const answer = await query(conn, alice, contract, "echo", [contract.address, foo, voyager, flags]);
        expect(answer.output?.toJSON()).toStrictEqual(voyager);
    });

    it('fails without the reentry flag', async function () {
        const flags = [CallFlags.TAIL_CALL];
        const answer = await query(conn, alice, contract, "echo", [contract.address, foo, voyager, flags]);
        const { index, error } = answer.result.asErr.asModule;
        // Module 8 error 0x16 is ReentranceDenied in the contracts pallet
        expect(index.toJSON()).toStrictEqual(8);
        expect(error.toJSON()).toStrictEqual("0x16000000");
    });

    it('fails with the input forwarding flag', async function () {
        const flags = [CallFlags.ALLOW_REENTRY, CallFlags.FORWARD_INPUT];
        const answer = await query(conn, alice, contract, "echo", [contract.address, foo, voyager, flags]);
        expect(answer.result.asOk.flags.isRevert).toStrictEqual(true);
    });

    it('test for the tail call flag to work correctly', async function () {
        let flags = [CallFlags.ALLOW_REENTRY];
        let answer = await query(conn, alice, contract, "tail_call_it", [contract.address, foo, voyager, flags]);
        expect(answer.output?.toJSON()).toStrictEqual(voyager + 1);

        flags = [CallFlags.ALLOW_REENTRY, CallFlags.TAIL_CALL];
        answer = await query(conn, alice, contract, "tail_call_it", [contract.address, foo, voyager, flags]);
        expect(answer.output?.toJSON()).toStrictEqual(voyager);
    });

    it('works on calls on "this"', async function () {
        const answer = await query(conn, alice, contract, "call_this", [voyager]);
        expect(answer.output?.toJSON()).toStrictEqual(voyager);
    });
});
