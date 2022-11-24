import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';

describe('Deploy external_call contract and test', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('external_call', async function () {
        this.timeout(100000);

        const alice = aliceKeypair();

        // call the constructors
        let caller_res = await deploy(conn, alice, 'caller.contract', BigInt(0));

        let caller_contract = new ContractPromise(conn, caller_res.abi, caller_res.address);

        let callee_res = await deploy(conn, alice, 'callee.contract', BigInt(0));

        let callee_contract = new ContractPromise(conn, callee_res.abi, callee_res.address);

        let callee2_res = await deploy(conn, alice, 'callee2.contract', BigInt(0));

        let callee2_contract = new ContractPromise(conn, callee2_res.abi, callee2_res.address);

        var gasLimit = await weight(conn, callee_contract, "setX", [102]);
        let tx1 = callee_contract.tx.setX({ gasLimit }, 102);

        await transaction(tx1, alice);

        let res1 = await query(conn, alice, callee_contract, "getX");

        expect(res1.output?.toJSON()).toStrictEqual(102);

        let res2 = await query(conn, alice, caller_contract, "whoAmI");

        expect(res2.output?.toString()).toEqual(caller_res.address.toString());

        var gasLimit = await weight(conn, caller_contract, "doCall", [callee_contract.address, 13123]);
        let tx2 = caller_contract.tx.doCall({ gasLimit }, callee_contract.address, 13123);

        await transaction(tx2, alice);

        let res3 = await query(conn, alice, callee_contract, "getX");

        expect(res3.output?.toJSON()).toStrictEqual(13123);

        let res4 = await query(conn, alice, caller_contract, "doCall2", [callee_contract.address, 20000]);

        expect(res4.output?.toJSON()).toStrictEqual(33123);

        let res5 = await query(conn, alice, caller_contract, "doCall3", [callee_contract.address, callee2_contract.address, [3, 5, 7, 9], "yo"]);

        expect(res5.output?.toJSON()).toEqual([24, "my name is callee"]);

        let res6 = await query(conn, alice, caller_contract, "doCall4", [callee_contract.address, callee2_contract.address, [1, 2, 3, 4], "asda"]);

        expect(res6.output?.toJSON()).toEqual([10, "x:asda"]);
    });
});
