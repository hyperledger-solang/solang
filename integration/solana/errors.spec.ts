import expect from 'expect';
import { establishConnection } from './index';

describe('Deploy solang contract and test', () => {
    it('errors', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        let errors = await conn.loadProgram("bundle.so", "errors.abi");

        // call the constructor
        await errors.call_constructor(conn, 'errors', []);

        let res = await errors.call_function(conn, "do_revert", [false]);

        expect(res["0"]).toBe("3124445");

        let revert_res = await errors.call_function_expect_revert(conn, "do_revert", [true]);

        expect(revert_res).toBe("Do the revert thing");

    });
});
