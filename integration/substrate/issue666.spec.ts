import expect from 'expect';
import { gasLimit, createConnection, deploy, transaction, aliceKeypair, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';

describe('issue666 flip and inc', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('tests for issue #666', async function () {
        this.timeout(50000);

        const alice = aliceKeypair();

        // call the constructors
        let flipper_contract = await deploy(conn, alice, 'Flip.contract', BigInt(0));
        let inc_contract = await deploy(conn, alice, 'Inc.contract', BigInt(0), flipper_contract.address);

        let ss58_addr = flipper_contract.address.toString();
        await deploy(conn, alice, 'Inc.contract', BigInt(0), ss58_addr);

        let contract = new ContractPromise(conn, inc_contract.abi, inc_contract.address);

        contract.tx.superFlip({ gasLimit });
    });
});
