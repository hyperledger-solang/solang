import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { DecodedEvent } from '@polkadot/api-contract/types';

describe('Deploy events contract and test event data, docs and topics', () => {
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

        let deploy_contract = await deploy(conn, alice, 'Events.contract', BigInt(0));
        let contract = new ContractPromise(conn, deploy_contract.abi, deploy_contract.address);
        let gasLimit = await weight(conn, contract, "emitEvent");
        let tx = contract.tx.emitEvent({ gasLimit });
        let res0: any = await transaction(tx, alice);
        let events: DecodedEvent[] = res0.contractEvents;

        expect(events.length).toEqual(4);

        expect(events[0].event.identifier).toBe("Events::foo1");
        expect(events[0].event.docs).toEqual(["Ladida tada"]);
        expect(events[0].args.map(a => a.toJSON())).toEqual([254, "hello there"]);

        expect(events[1].event.identifier).toBe("Events::foo2");
        expect(events[1].event.docs).toEqual(["Event Foo2\n\nJust a test\n\nAuthor: them is me"]);
        expect(events[1].args.map(a => a.toJSON())).toEqual(["0x7fffffffffffffff", "minor", deploy_contract.address.toString()]);

        expect(events[2].event.identifier).toBe("Events::ThisEventTopicShouldGetHashed");
        expect(events[2].args.map(a => a.toJSON())).toEqual([alice.address]);

        // Expect the 3rd event to yield the following event topics:
        // - blake2x256 sum of its signature: 'ThisEventTopicShouldGetHashed(address)'
        // - Address of the caller

        let field_topic = await conn.query.system.eventTopics(alice.addressRaw);
        expect(field_topic.length).toBe(1);

        let event_topic = await conn.query.system.eventTopics("0x95c29b3e1b835071ab157a22d89cfc81d176add91127a1ee8766abf406a2cbc3");
        expect(event_topic.length).toBe(1);

        expect(events[3].event.identifier).toBe("Events::Event");
        expect(events[3].args.map(a => a.toJSON())).toEqual([true]);

        // The 4th event yields the following event topics:
        // - blake2x256 sum of its signature: 'Event(bool)'
        // - unhashed data (because encoded length is <= 32 bytes) of 'true'

        field_topic = await conn.query.system.eventTopics("0x0100000000000000000000000000000000000000000000000000000000000000");
        expect(field_topic.length).toBe(1);
        event_topic = await conn.query.system.eventTopics("0xc2bc7a077121efada8bc6a0af16c1e886406e8c2d1716979cb1b92098d8b49bc");
        expect(event_topic.length).toBe(1);
    });
});
