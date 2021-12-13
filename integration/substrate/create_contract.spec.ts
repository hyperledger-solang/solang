import expect from 'expect';
import { gasLimit, createConnection, deploy, transaction, aliceKeypair, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';

describe('Deploy create_contract contract and test', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('create_contract', async function () {
        this.timeout(50000);

        const alice = aliceKeypair();

        // call the constructors
        let deploy_contract = await deploy(conn, alice, 'creator.contract');

        // we need to have upload the child code
        let _ = await deploy(conn, alice, 'child.contract');

        let contract = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);

        let tx = contract.tx.createChild({ gasLimit });

        await transaction(tx, alice);

        let res2 = await contract.query.callChild(alice.address, {});

        expect(res2.output?.toJSON()).toStrictEqual("child");

        // child was created with a balance of 1e15, verify
        res2 = await contract.query.c(alice.address, {});

        let child = res2.output!.toString();

        let { data: { free: childBalance } } = await conn.query.system.account(child);

        expect(BigInt(1e15) - childBalance.toBigInt()).toBeLessThan(1e11);
    });
});
