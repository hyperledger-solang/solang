import expect from 'expect';
import { createConnection, deploy, transaction, aliceKeypair, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';

describe('Deploy contract with overloaded functions using mangled names', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('works with mangled function names', async function () {
        this.timeout(50000);

        const alice = aliceKeypair();
        let deploy_contract = await deploy(conn, alice, 'Overloading.contract', BigInt(0));
        let contract = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);

        let res0 = await query(conn, alice, contract, "echo");
        expect(res0.output?.toJSON()).toEqual(42);

        let res1 = await query(conn, alice, contract, "echoUint32", [1234]);
        expect(res1.output?.toJSON()).toEqual(1234);

        let someStruct = { s: "foo", e: [["v1", "v2"], ["v3", "v4"]] };
        let res2 = await query(conn, alice, contract, "echoBoolStringUint8Array2Array", [true, someStruct]);
        expect(res2.output?.toJSON()).toEqual(someStruct);
    });
});
