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

        expect(events[0].event.identifier).toBe("foo1");
        expect(events[0].event.docs).toEqual(["Ladida tada"]);
        expect(events[0].args.map(a => a.toJSON())).toEqual([254, "hello there"]);

        expect(events[1].event.identifier).toBe("foo2");
        expect(events[1].event.docs).toEqual(["Event Foo2\n\nJust a test\n\nAuthor: them is me"]);
        expect(events[1].args.map(a => a.toJSON())).toEqual(["0x7fffffffffffffff", "minor", deploy_contract.address.toString()]);

        expect(events[2].event.identifier).toBe("ThisEventTopicShouldGetHashed");
        expect(events[2].args.map(a => a.toJSON())).toEqual([alice.address]);

        // In ink! the 3rd event does look like this:
        //
        //  #[ink(event)]
        //  pub struct ThisEventTopicShouldGetHashed {
        //      #[ink(topic)]
        //      caller: AccountId,
        //  }
        //
        // It yields the following event topics:
        //
        //  topics: [
        //      0x5dde952854d38c37cff349bfc574a48a831de385b82457a5c25d9d39c220f3a7
        //      0xa5af79de4a26a64813f980ffbb64ac0d7c278f67b17721423daed26ec5d3fe51
        //  ]
        //
        // So we expect our solidity contract to produce the exact same topics:

        let hashed_event_topics = await conn.query.system.eventTopics("0x5dde952854d38c37cff349bfc574a48a831de385b82457a5c25d9d39c220f3a7");
        expect(hashed_event_topics.length).toBe(1);
        let hashed_topics = await conn.query.system.eventTopics("0xa5af79de4a26a64813f980ffbb64ac0d7c278f67b17721423daed26ec5d3fe51");
        expect(hashed_topics.length).toBe(1);

        expect(events[3].event.identifier).toBe("Event");
        expect(events[3].args.map(a => a.toJSON())).toEqual([true]);

        // In ink! the 4th event does look like this:
        //
        //  #[ink(event)]
        //  pub struct Event {
        //      #[ink(topic)]
        //      something: bool,
        //  }
        //
        // It yields the following event topics:
        //
        //  topics: [
        //      0x004576656e74733a3a4576656e74000000000000000000000000000000000000
        //      0x604576656e74733a3a4576656e743a3a736f6d657468696e6701000000000000
        //  ]
        //
        // So we expect our solidity contract to produce the exact same topics:

        let unhashed_event_topics = await conn.query.system.eventTopics("0x004576656e74733a3a4576656e74000000000000000000000000000000000000");
        expect(unhashed_event_topics.length).toBe(1);
        let unhashed_topics = await conn.query.system.eventTopics("0x604576656e74733a3a4576656e743a3a736f6d657468696e6701000000000000");
        expect(unhashed_topics.length).toBe(1);
    });
});
