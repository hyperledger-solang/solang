import expect from 'expect';
import { gasLimit, createConnection, deploy, transaction, aliceKeypair, daveKeypair } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';

describe('Deploy destruct contract and test', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('destruct', async function () {
        this.timeout(50000);

        const alice = aliceKeypair();
        const dave = daveKeypair();

        // call the constructors
        let deploy_contract = await deploy(conn, alice, 'destruct.contract', BigInt(0));

        let contract = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);

        let hello = await contract.query.hello(alice.address, {});

        expect(hello.output?.toJSON()).toBe('Hello');

        let { data: { free: daveBalBefore } } = await conn.query.system.account(dave.address);
        let { data: { free: contractBalBefore } } = await conn.query.system.account(String(deploy_contract.address));

        let tx = contract.tx.selfterminate({ gasLimit }, dave.address);

        await transaction(tx, alice);

        let { data: { free: daveBalAfter } } = await conn.query.system.account(dave.address);
        let { data: { free: contractBalAfter } } = await conn.query.system.account(String(deploy_contract.address));

        //console.log(`bal ${daveBalBefore} and ${daveBalAfter}`);
        //console.log(`bal ${contractBalBefore} and ${contractBalAfter}`);

        // The contact is gone and has no balance
        expect(contractBalAfter.toBigInt()).toBe(0n);
        // Dave now has the balance previously held by the contract
        expect(daveBalAfter.toBigInt()).toEqual(daveBalBefore.toBigInt() + contractBalBefore.toBigInt());
    });
});
