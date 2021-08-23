import expect from 'expect';
import { establishConnection } from './index';

describe('Deploy solang contract and test', () => {
    it('Events', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        let hash_functions = await conn.loadProgram("bundle.so", "Events.abi");

        // call the constructor
        await hash_functions.call_constructor(conn, 'Events', []);

        let res = await hash_functions.call_function(conn, "getName", []);

        expect(res["0"]).toBe("myName");

        await hash_functions.call_function(conn, "setName", ['ozan']);

        res = await hash_functions.call_function(conn, "getName", []);

        expect(res["0"]).toBe('ozan');

        await hash_functions.call_function(conn, "setSurname", ['martin']);

        res = await hash_functions.call_function(conn, "getSurname", []);

        expect(res["0"]).toBe('martin');

        res = await hash_functions.call_function(conn, "getNames", []);

        expect(res["0"]).toBe('ozan');
        expect(res["1"]).toBe('martin');
    });
});
