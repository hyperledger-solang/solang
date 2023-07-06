import expect from 'expect';
import { createConnection, deploy, transaction, aliceKeypair, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';

describe('Deploy builtin contract and test', () => {
    it('builtins', async function () {
        this.timeout(50000);

        let conn = await createConnection();
        const alice = aliceKeypair();

        let deployed_contract = await deploy(conn, alice, 'builtins.contract', BigInt(0));

        let contract = new ContractPromise(conn, deployed_contract.abi, deployed_contract.address);

        // call the constructor
        let ripemd160_res = await query(conn, alice, contract, "hashRipemd160", ['0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex')]);

        expect(ripemd160_res.output?.toJSON()).toBe("0x0c8b641c461e3c7abbdabd7f12a8905ee480dadf");

        let sha256_res = await query(conn, alice, contract, "hashSha256", ['0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex')]);

        expect(sha256_res.output?.toJSON()).toBe("0x458f3ceeeec730139693560ecf66c9c22d9c7bc7dcb0599e8e10b667dfeac043");

        let keccak256_res = await query(conn, alice, contract, "hashKecccak256", ['0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex')]);

        expect(keccak256_res.output?.toJSON()).toBe("0x823ad8e1757b879aac338f9a18542928c668e479b37e4a56f024016215c5928c");

        let timestamps_res = await query(conn, alice, contract, "mrNow");

        let now = Math.floor(+new Date() / 1000);

        let ts = Number(timestamps_res.output?.toJSON());

        expect(ts).toBeLessThanOrEqual(now);
        expect(ts).toBeGreaterThan(now - 120);

        conn.disconnect();
    });
});
