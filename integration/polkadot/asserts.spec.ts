import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';

describe('Deploy asserts contract and test', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('asserts', async function () {
        this.timeout(50000);

        const alice = aliceKeypair();

        // call the constructors
        let deploy_contract = await deploy(conn, alice, 'asserts.contract', BigInt(0));

        let contract = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);

        let res0 = await query(conn, alice, contract, "var");
        expect(res0.output?.toJSON()).toEqual(1);

        let res1 = await query(conn, alice, contract, "testAssertRpc");
        expect(res1.result.asOk.flags.isRevert).toStrictEqual(true);
        expect(res1.result.asOk.data.toString()).toStrictEqual("0x08c379a0204920726566757365");

        res1 = await query(conn, alice, contract, "testAssert");
        expect(res1.result.asOk.flags.isRevert).toStrictEqual(true);
        expect(res1.result.asOk.data.toString()).toStrictEqual("0x4e487b710100000000000000000000000000000000000000000000000000000000000000");

        let gasLimit = await weight(conn, contract, "testAssert");
        let tx = contract.tx.testAssert({ gasLimit });

        let res2 = await transaction(tx, alice).then(() => {
            throw new Error("should not succeed");
        }, (res) => res);

        // Error 25 is ContractReverted
        expect(res2.dispatchError.toHuman()).toEqual({ "Module": { "error": "0x19000000", "index": "8" } });

        let res3 = await query(conn, alice, contract, "var");

        expect(res3.output?.toJSON()).toEqual(1);
    });
});
