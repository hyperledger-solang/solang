import expect from 'expect';
import { establishConnection } from './index';

describe('Deploy solang contract and test', () => {
    it('flipper', async function () {
        this.timeout(10000);

        let conn = await establishConnection();

        let prog = await conn.loadProgram("flipper.so", "flipper.abi");

        // call the constructor
        await prog.call_constructor(conn, ["true"]);

        let res = await prog.call_function(conn, "get", []);

        expect(res["__length__"]).toBe(1);
        expect(res["0"]).toBe(true);

        await prog.call_function(conn, "flip", []);

        res = await prog.call_function(conn, "get", []);

        expect(res["__length__"]).toBe(1);
        expect(res["0"]).toBe(false);

    });
});
