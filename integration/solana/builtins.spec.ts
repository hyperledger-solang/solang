import expect from 'expect';
import { establishConnection } from './index';
import crypto from 'crypto';

describe('Deploy solang contract and test', () => {
    it('builtins', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        let hash_functions = await conn.loadProgram("builtins.so", "builtins.abi");

        // call the constructor
        await hash_functions.call_constructor(conn, 'builtins', []);

        console.log("calling ripemd160");
        let res = await hash_functions.call_function(conn, "hash_ripemd160", ['0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex')]);

        expect(res["0"]).toBe("0x0c8b641c461e3c7abbdabd7f12a8905ee480dadf");

        console.log("calling sha256");
        res = await hash_functions.call_function(conn, "hash_sha256", ['0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex')]);

        expect(res["0"]).toBe("0x458f3ceeeec730139693560ecf66c9c22d9c7bc7dcb0599e8e10b667dfeac043");

        console.log("calling keccak256");
        res = await hash_functions.call_function(conn, "hash_kecccak256", ['0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex')]);

        expect(res["0"]).toBe("0x823ad8e1757b879aac338f9a18542928c668e479b37e4a56f024016215c5928c");

        console.log("calling timestamp");
        res = await hash_functions.call_function(conn, "mr_now", []);

        let now = Math.floor(+new Date() / 1000);

        let ts = Number(res[0]);

        expect(ts).toBeLessThanOrEqual(now);
        expect(ts).toBeGreaterThan(now - 120);
    });
});
