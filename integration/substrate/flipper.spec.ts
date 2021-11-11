import expect from 'expect';
import { gasLimit, createConnection, deploy, transaction, aliceKeypair, } from './index';
import { ContractPromise } from '@polkadot/api-contract';

describe('Deploy flipper contract and test', () => {
    it('flipper', async function () {
        this.timeout(50000);

        let conn = await createConnection();
        const alice = aliceKeypair();

        let deployed_contract = await deploy(conn, alice, 'flipper.contract', true);

        let contract = new ContractPromise(conn, deployed_contract.abi, deployed_contract.address);

        let init_value = await contract.query.get(alice.address, {});

        expect(init_value.output?.toJSON()).toBe(true);

        const tx = contract.tx.flip({ gasLimit });

        await transaction(tx, alice);

        let flipped_value = await contract.query.get(alice.address, {});

        expect(flipped_value.output?.toJSON()).toBe(false);

        conn.disconnect();
    });
});
