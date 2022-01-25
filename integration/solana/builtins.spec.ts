import expect from 'expect';
import { loadContract } from './setup';

describe('Deploy solang contract and test', function () {
    this.timeout(500000);

    it('builtins', async function () {
        let { contract, connection } = await loadContract('builtins', 'builtins.abi');

        // call the constructor
        let res = await contract.functions.hash_ripemd160('0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex'));

        expect(res.result).toBe("0x0c8b641c461e3c7abbdabd7f12a8905ee480dadf");

        res = await contract.functions.hash_sha256('0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex'), { simulate: true });

        expect(res.result).toBe("0x458f3ceeeec730139693560ecf66c9c22d9c7bc7dcb0599e8e10b667dfeac043");

        res = await contract.functions.hash_kecccak256('0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex'));

        expect(res.result).toBe("0x823ad8e1757b879aac338f9a18542928c668e479b37e4a56f024016215c5928c");

        res = await contract.functions.mr_now([]);

        let now = Math.floor(+new Date() / 1000);

        let ts = Number(res.result);

        expect(ts).toBeLessThanOrEqual(now);
        expect(ts).toBeGreaterThan(now - 120);

        res = await contract.functions.mr_slot();

        let sol_slot = Number(res.result);

        let rpc_slot = await connection.getSlot();

        expect(sol_slot).toBeGreaterThan(rpc_slot - 10);
        expect(sol_slot).toBeLessThan(rpc_slot + 10);
    });
});
