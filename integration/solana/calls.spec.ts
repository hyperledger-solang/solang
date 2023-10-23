// SPDX-License-Identifier: Apache-2.0

import expect from 'expect';
import { loadContractAndCallConstructor, loadContractWithProvider } from './setup';
import { BN } from '@coral-xyz/anchor';

describe('Testing calls', function () {
    this.timeout(100000);

    it('external_call', async function () {
        let caller = await loadContractAndCallConstructor('caller');

        const provider = caller.provider;

        const callee = await loadContractWithProvider(provider, 'callee');

        const callee2 = await loadContractWithProvider(provider, 'callee2');

        await callee.program.methods.setX(new BN(102))
            .accounts({ dataAccount: callee.storage.publicKey })
            .rpc({commitment: "processed"});

        let res = await callee.program.methods.getX()
            .accounts({ dataAccount: callee.storage.publicKey })
            .view({commitment: "processed"});

        expect(res).toEqual(new BN(102));

        res = await caller.program.methods.whoAmI()
            .view({commitment: "processed"});

        expect(res).toStrictEqual(caller.program_key);

        await caller.program.methods.doCall(new BN(13123))
            .accounts({
                callee_dataAccount: callee.storage.publicKey,
                callee_pid: callee.program_key,
                })
            .rpc({commitment: "processed"});

        res = await callee.program.methods.getX()
            .accounts({ dataAccount: callee.storage.publicKey })
            .view({commitment: "processed"});

        expect(res).toEqual(new BN(13123));

        res = await caller.program.methods.doCall2(new BN(20000))
            .accounts({
                callee_dataAccount: callee.storage.publicKey,
                callee_pid: callee.program_key,
            })
            .view({commitment: "processed"});

        expect(res).toEqual(new BN(33123));

        res = await caller.program.methods.doCall3([new BN(3), new BN(5), new BN(7), new BN(9)], "yo")
            .accounts({
                callee2_pid: callee2.program_key,
                callee_pid: callee.program_key,
            })
            .view({commitment: "processed"});

        expect(res.return0).toEqual(new BN(24));
        expect(res.return1).toBe("my name is callee");

        res = await caller.program.methods.doCall4([new BN(1), new BN(2), new BN(3), new BN(4)], "asda")
            .accounts({
                callee2_pid: callee2.program_key,
                callee_pid: callee.program_key,
                other_callee2: callee2.program_key,
            })
            .view({commitment: "processed"});

        expect(res.return0).toEqual(new BN(10));
        expect(res.return1).toBe("x:asda");
    });
});
