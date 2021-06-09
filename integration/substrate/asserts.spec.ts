import expect from 'expect';
import { gasLimit, createConnection, deploy, transaction, aliceKeypair, } from './index';
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
        let deploy_contract = await deploy(conn, alice, 'asserts.contract');

        let contract = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);

        let res0 = await contract.query.var(alice.address, {});

        expect(res0.output?.toJSON()).toEqual(1);

        let res1 = await contract.query.testAssertRpc(alice.address, {});
        expect(res1.result.toHuman()).toEqual({ "Err": { "Module": { "error": "19", "index": "18", "message": "ContractTrapped" } } });

        let tx = contract.tx.testAssert({ gasLimit });

        let res2 = await transaction(tx, alice).then(() => {
            throw new Error("should not succeed");
        }, (res) => res);

        expect(res2.dispatchError.toHuman()).toEqual({ "Module": { "error": "19", "index": "18" } });

        let res3 = await contract.query.var(alice.address, {});

        expect(res3.output?.toJSON()).toEqual(1);
    });
});
