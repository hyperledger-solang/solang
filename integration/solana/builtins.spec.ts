// SPDX-License-Identifier: Apache-2.0

import expect from 'expect';
import { loadContract } from './setup';
import { AccountMeta, SYSVAR_CLOCK_PUBKEY, PublicKey } from '@solana/web3.js';

describe('Testing builtins', function () {
    this.timeout(500000);

    it('builtins', async function () {
        let { program, provider, storage } = await loadContract('builtins');

        // call the constructor
        let res = await program.methods.hashRipemd160(Buffer.from('Call me Ishmael.', 'utf8')).view();
        expect(Buffer.from(res).toString("hex")).toBe("0c8b641c461e3c7abbdabd7f12a8905ee480dadf");

        res = await program.methods.hashSha256(Buffer.from('Call me Ishmael.', 'utf8')).view();
        expect(Buffer.from(res).toString("hex")).toBe("458f3ceeeec730139693560ecf66c9c22d9c7bc7dcb0599e8e10b667dfeac043");

        res = await program.methods.hashKecccak256(Buffer.from('Call me Ishmael.', 'utf8')).view();
        expect(Buffer.from(res).toString("hex")).toBe("823ad8e1757b879aac338f9a18542928c668e479b37e4a56f024016215c5928c");

        let addrs = new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111");
        const expected_pubkey = new PublicKey("BwqrghZA2htAcqq8dzP1WDAhTXYTYWj7CHxF5j7TDBAe");
        res = await program.methods.pda(Buffer.from([]), Buffer.from([1]), addrs).view();
        expect(res).toEqual(expected_pubkey);

        res = await program.methods.pdaWithBump(Buffer.from([]), Buffer.from([1]), addrs).view();
        expect(res['return0']).toEqual(new PublicKey("13wtuiEKKtsFgwPmwwtgnELMyK7E8s1bcA8wJi6hFn5A"));
        expect(res['return1']).toEqual([0xfe]);

        let clock: AccountMeta[] = [{
            pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false
        }];

        res = await program.methods.mrNow()
            .accounts({ dataAccount: storage.publicKey })
            .remainingAccounts(clock)
            .view();

        let now = Math.floor(+new Date() / 1000);

        let ts = Number(res);

        expect(ts).toBeLessThanOrEqual(now);
        expect(ts).toBeGreaterThan(now - 120);

        res = await program.methods.mrSlot()
            .accounts({ dataAccount: storage.publicKey })
            .remainingAccounts(clock)
            .view();

        let sol_slot = Number(res);

        let rpc_slot = await provider.connection.getSlot();

        expect(sol_slot).toBeGreaterThan(rpc_slot - 10);
        expect(sol_slot).toBeLessThan(rpc_slot + 10);
    });
});
