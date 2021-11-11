import expect from 'expect';
import { loadContract } from './utils';

describe('Deploy solang contract and test', () => {
    it('builtins', async function () {
        this.timeout(50000);

        let [token, connection] = await loadContract('builtins', 'builtins.abi');

        // call the constructor
        console.log("calling ripemd160");
        let res = await token.functions.hash_ripemd160('0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex'));

        expect(res.result).toBe("0x0c8b641c461e3c7abbdabd7f12a8905ee480dadf");

        console.log("calling sha256");
        res = await token.functions.hash_sha256('0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex'), { simulate: true });

        expect(res.result).toBe("0x458f3ceeeec730139693560ecf66c9c22d9c7bc7dcb0599e8e10b667dfeac043");

        console.log("calling keccak256");
        res = await token.functions.hash_kecccak256('0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex'));

        expect(res.result).toBe("0x823ad8e1757b879aac338f9a18542928c668e479b37e4a56f024016215c5928c");

        console.log("calling timestamp");
        res = await token.functions.mr_now([]);

        let now = Math.floor(+new Date() / 1000);

        let ts = Number(res.result);

        expect(ts).toBeLessThanOrEqual(now);
        expect(ts).toBeGreaterThan(now - 120);

        console.log("calling slot");
        res = await token.functions.mr_slot();

        let sol_slot = Number(res.result);

        let rpc_slot = await connection.getSlot();
        console.log("slot from rpc " + rpc_slot);

        expect(sol_slot).toBeGreaterThan(rpc_slot - 10);
        expect(sol_slot).toBeLessThan(rpc_slot + 10);
    });
});
