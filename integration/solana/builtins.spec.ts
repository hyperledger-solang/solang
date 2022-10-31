import expect from 'expect';
import { loadContract } from './setup';
import { PublicKey } from '@solana/web3.js';

describe('Testing builtins', function () {
    this.timeout(500000);

    it('builtins', async function () {
        let { contract, connection } = await loadContract('builtins', 'builtins.abi');

        // call the constructor
        let res = await contract.functions.hash_ripemd160(new Uint8Array(Buffer.from('Call me Ishmael.', 'utf8')));
        expect(Buffer.from(res.result).toString("hex")).toBe("0c8b641c461e3c7abbdabd7f12a8905ee480dadf");

        res = await contract.functions.hash_sha256(new Uint8Array(Buffer.from('Call me Ishmael.', 'utf8')), { simulate: true });
        expect(Buffer.from(res.result).toString("hex")).toBe("458f3ceeeec730139693560ecf66c9c22d9c7bc7dcb0599e8e10b667dfeac043");

        res = await contract.functions.hash_kecccak256(new Uint8Array(Buffer.from('Call me Ishmael.', 'utf8')));
        expect(Buffer.from(res.result).toString("hex")).toBe("823ad8e1757b879aac338f9a18542928c668e479b37e4a56f024016215c5928c");

        let addrs = new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111");
        const expected_pubkey = new PublicKey("BwqrghZA2htAcqq8dzP1WDAhTXYTYWj7CHxF5j7TDBAe");
        res = await contract.functions.pda(new Uint8Array(), new Uint8Array([1]), addrs.toBytes());
        expect(Buffer.from(res.result)).toEqual(expected_pubkey.toBytes());

        res = await contract.functions.pda_with_bump(new Uint8Array(), new Uint8Array([1]), addrs.toBytes());
        expect(Buffer.from(res.result[0]).toString("hex")).toBe("00c13b4820057d4d07cddf24058df4034cd3379a5f863b4e061ad2c29be62fd5");
        expect(Buffer.from(res.result[1]).toString("hex")).toBe("fe");

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
