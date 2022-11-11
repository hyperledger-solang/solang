import expect from 'expect';
import { gasLimit, createConnection, deploy, transaction, aliceKeypair, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { DecodedEvent } from '@polkadot/api-contract/types';

describe('Tests for event topics', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('creates correct topics', async function () {
        this.timeout(50000);

        const alice = aliceKeypair();
        let deploy_contract = await deploy(conn, alice, 'event_topics.contract', 0n);
        let contract = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);

        let tx = contract.tx.foo({ gasLimit }, "name1", 123);
        let res0: any = await transaction(tx, alice);
        let events: DecodedEvent[] = res0.contractEvents;
        /// XXX: This does not work with indexed fields
        expect(events.length).toEqual(1);
    });
});
