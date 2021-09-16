import expect from 'expect';
import { establishConnection } from './index';

describe('Deploy solang contract and test', () => {
    it('events', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        let event_contract = await conn.loadProgram("bundle.so", "events.abi");

        // call the constructor
        await event_contract.call_constructor(conn, 'events', []);

        let events = await event_contract.call_function_events(conn, "test", []);

        expect(events[0]["0"]).toEqual("102");
        expect(events[0]["1"]).toEqual(true);
        expect(events[0]["2"]).toEqual("foobar");

        expect(events[1]["0"]).toEqual("500332");
        expect(events[1]["1"]).toEqual("0x41424344");
        expect(events[1]["2"]).toEqual("0xcafe0123");
    });
});
