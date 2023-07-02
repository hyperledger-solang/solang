import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';

describe('Deploy flipper contract and test', () => {
    it('flipper', async function () {
        this.timeout(50000);

        let conn = await createConnection();
        const alice = aliceKeypair();

        let deployed_contract = await deploy(conn, alice, 'flipper.contract', BigInt(0), true);

        let contract = new ContractPromise(conn, deployed_contract.abi, deployed_contract.address);

        let init_value = await query(conn, alice, contract, "get");

        expect(init_value.output?.toJSON()).toBe(true);

        let gasLimit = await weight(conn, contract, "flip");
        const tx = contract.tx.flip({ gasLimit });

        await transaction(tx, alice);

        let flipped_value = await query(conn, alice, contract, "get");

        expect(flipped_value.output?.toJSON()).toBe(false);

        conn.disconnect();
    });
});
