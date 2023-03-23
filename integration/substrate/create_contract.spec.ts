import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, query, debug_buffer, dry_run, } from './index';
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
        let deploy_contract = await deploy(conn, alice, 'creator.contract', BigInt(1e16));

        // we need to have upload the child code
        let _ = await deploy(conn, alice, 'child_create_contract.contract', BigInt(0));

        let contract = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);

        let dry = await dry_run(conn, contract, "createChild");
        // Expect the instantiation nonce to be present
        let current_nonce = dry.debugMessage.split('\n')[0].split('=');
        expect(current_nonce[0]).toEqual("call: instantiation_nonce");

        let gasLimit = dry.gasRequired;
        let tx = contract.tx.createChild({ gasLimit });

        await transaction(tx, alice);

        let res2 = await query(conn, alice, contract, "callChild");

        expect(res2.output?.toJSON()).toStrictEqual("child");

        // child was created with a balance of 1e15, verify
        res2 = await query(conn, alice, contract, "c");

        let child = res2.output!.toString();

        let { data: { free: childBalance } } = await conn.query.system.account(child);

        expect(BigInt(1e15) - childBalance.toBigInt()).toBeLessThan(1e11);

        // Expect the instantiation nonce to be present and to increase
        let next_nonce = (await debug_buffer(conn, contract, "createChild")).split('\n')[0].split('=');
        expect(next_nonce[0]).toEqual("call: instantiation_nonce");
        expect(parseInt(current_nonce[1])).toBeLessThan(parseInt(next_nonce[1]));
    });
});
