import expect from 'expect';
import { gasLimit, createConnection, deploy, transaction, aliceKeypair, daveKeypair } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';

describe('Deploy balances contract and test', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('balances', async function () {
        this.timeout(50000);

        const alice = aliceKeypair();
        const dave = daveKeypair();

        // call the constructors
        let deploy_contract = await deploy(conn, alice, 'balances.contract', BigInt(1e7));

        let contract = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);

        let { output: contractRpcBal } = await contract.query.getBalance(alice.address, {});
        let { data: { free: contractQueryBalBefore } } = await conn.query.system.account(String(deploy_contract.address));

        expect(contractRpcBal?.toString()).toBe(contractQueryBalBefore.toString());

        let tx = contract.tx.payMe({ gasLimit, value: 1000000n });

        await transaction(tx, alice);

        let { data: { free: contractQueryBalAfter } } = await conn.query.system.account(String(deploy_contract.address));

        expect(contractQueryBalAfter.toBigInt()).toEqual(contractQueryBalBefore.toBigInt() + 1000000n);

        let { data: { free: daveBal1 } } = await conn.query.system.account(dave.address);

        let tx1 = contract.tx.transfer({ gasLimit }, dave.address, 20000);

        await transaction(tx1, alice);

        let { data: { free: daveBal2 } } = await conn.query.system.account(dave.address);

        expect(daveBal2.toBigInt()).toEqual(daveBal1.toBigInt() + 20000n);

        let tx2 = contract.tx.send({ gasLimit }, dave.address, 10000);

        await transaction(tx2, alice);

        let { data: { free: daveBal3 } } = await conn.query.system.account(dave.address);

        expect(daveBal3.toBigInt()).toEqual(daveBal2.toBigInt() + 10000n);
    });
});
