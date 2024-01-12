import expect from 'expect';
import { createConnection, deploy, aliceKeypair, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { daveKeypair } from './index';

describe('Test the FetchRandom chain extension from the ink! examples', () => {
    it('Calls the FetchRandom output multiple times and tests the output ', async function () {
        this.timeout(50000);

        const conn = await createConnection();
        const alice = aliceKeypair();

        const deployed_contract = await deploy(conn, alice, 'ChainExtension.contract', 0n);
        const contract = new ContractPromise(conn, deployed_contract.abi, deployed_contract.address);

        const seeded_with_alice_1 = await query(conn, alice, contract, "fetchRandom", [alice.addressRaw]);
        const seeded_with_dave = await query(conn, alice, contract, "fetchRandom", [daveKeypair().addressRaw]);
        const seeded_with_alice_2 = await query(conn, alice, contract, "fetchRandom", [alice.addressRaw]);

        expect(seeded_with_alice_1.output?.toJSON()).toEqual(seeded_with_alice_2.output?.toJSON());
        expect(seeded_with_alice_1.output?.toJSON()).not.toEqual(seeded_with_dave.output?.toJSON());

        conn.disconnect();
    });
});
