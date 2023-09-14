import expect from 'expect';
import { createConnection, deploy, aliceKeypair, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';


describe('Deploy and test the try_catch contract', () => {
    let conn: ApiPromise;
    let caller: ContractPromise;
    let alice: KeyringPair;

    before(async function () {
        alice = aliceKeypair();

        conn = await createConnection();
        await deploy(conn, alice, 'TryCatchCallee.contract', 0n);
        const caller_contract = await deploy(conn, alice, 'TryCatchCaller.contract', 1000000000n);
        caller = new ContractPromise(conn, caller_contract.abi, caller_contract.address);
    });

    after(async function () {
        await conn.disconnect();
    });

    it('Tests all catch clauses', async function () {
        this.timeout(20000);

        for (let in_out = 0; in_out < 5; in_out++) {
            console.log("Testing case: " + in_out);
            const answer = await query(conn, alice, caller, "test", [in_out]);
            expect(answer.output?.toJSON()).toStrictEqual(in_out);
        }
    });
});
