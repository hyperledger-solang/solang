import expect from 'expect';
import { gasLimit, createConnection, deploy, transaction, aliceKeypair, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { DecodedEvent } from '@polkadot/api-contract/types';

describe('Deploy events contract and test', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('events', async function () {
        this.timeout(50000);

        const alice = aliceKeypair();

        // call the constructors
        let deploy_contract = await deploy(conn, alice, 'events.contract');

        let contract = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);

        let tx = contract.tx.emitEvent({ gasLimit });

        let res0: any = await transaction(tx, alice);

        let events: DecodedEvent[] = res0.contractEvents;

        expect(events.length).toEqual(2);

        expect(events[0].event.identifier).toBe("foo1");
        expect(events[0].event.docs).toEqual(["Ladida tada\n\n"]);
        expect(events[0].args.map(a => a.toJSON())).toEqual([254, "hello there"]);

        expect(events[1].event.identifier).toBe("foo2");
        expect(events[1].event.docs).toEqual(["Event Foo2\n\nJust a test\n\nAuthor: them is me"]);
        expect(events[1].args.map(a => a.toJSON())).toEqual(["0x7fffffffffffffff", "minor", deploy_contract.address.toString()]);
    });
});
