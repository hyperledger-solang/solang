import expect from 'expect';
import { gasLimit, createConnection, deploy, transaction, aliceKeypair, daveKeypair } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';

describe('Deploy randomizer contract and test', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('randomizer', async function () {
        this.timeout(50000);

        const alice = aliceKeypair();
        const dave = daveKeypair();

        // call the constructors
        let deploy_contract = await deploy(conn, alice, 'randomizer.contract');

        let contract = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);

        let { output: queryOutput } = await contract.query.getRandom(alice.address, {}, '01234567');

        let tx = contract.tx.getRandom({ gasLimit }, '01234567');

        await transaction(tx, alice);

        let { output: txOutput } = await contract.query.value(alice.address, {});

        let queryRandom = queryOutput!.toU8a();
        let txRandom = txOutput!.toU8a();

        expect(queryRandom.length).toBe(32);
        expect(txRandom.length).toBe(32);
        expect(txRandom).not.toBe(queryRandom);
        expect(queryRandom).not.toBe(Buffer.alloc(32));
        expect(txRandom).not.toBe(Buffer.alloc(32));
    });
});
