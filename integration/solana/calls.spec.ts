// SPDX-License-Identifier: Apache-2.0

import expect from 'expect';
import { loadContract, loadContractWithProvider } from './setup';
import { BN } from '@project-serum/anchor';

describe('Testing calls', function () {
    this.timeout(100000);

    it('external_call', async function () {
        let caller = await loadContract('caller');

        const provider = caller.provider;

        const callee = await loadContractWithProvider(provider, 'callee');

        const callee2 = await loadContractWithProvider(provider, 'callee2');

        await callee.program.methods.setX(new BN(102))
            .accounts({ dataAccount: callee.storage.publicKey })
            .rpc();

        let res = await callee.program.methods.getX()
            .accounts({ dataAccount: callee.storage.publicKey })
            .view();

        expect(res).toEqual(new BN(102));

        let address_caller = caller.storage.publicKey;
        let address_callee = callee.storage.publicKey;
        let address_callee2 = callee2.storage.publicKey;

        res = await caller.program.methods.whoAmI()
            .accounts({ dataAccount: caller.storage.publicKey })
            .view();

        expect(res).toStrictEqual(address_caller);

        await caller.program.methods.doCall(address_callee, new BN(13123))
            .accounts({ dataAccount: caller.storage.publicKey })
            .remainingAccounts([
                { pubkey: callee.storage.publicKey, isSigner: false, isWritable: true },
                { pubkey: callee.program_key, isSigner: false, isWritable: false },
            ])
            .rpc();

        res = await callee.program.methods.getX()
            .accounts({ dataAccount: callee.storage.publicKey })
            .view();

        expect(res).toEqual(new BN(13123));

        res = await caller.program.methods.doCall2(address_callee, new BN(20000))
            .accounts({ dataAccount: caller.storage.publicKey })
            .remainingAccounts([
                { pubkey: callee.storage.publicKey, isSigner: false, isWritable: true },
                { pubkey: callee.program_key, isSigner: false, isWritable: false },
                { pubkey: caller.program_key, isSigner: false, isWritable: false },
            ])
            .view();

        expect(res).toEqual(new BN(33123));

        let all_keys = [
            { pubkey: callee.storage.publicKey, isSigner: false, isWritable: true },
            { pubkey: callee.program_key, isSigner: false, isWritable: false },
            { pubkey: callee2.storage.publicKey, isSigner: false, isWritable: true },
            { pubkey: callee2.program_key, isSigner: false, isWritable: false },
        ];

        res = await caller.program.methods.doCall3(address_callee, address_callee2, [new BN(3), new BN(5), new BN(7), new BN(9)], "yo")
            .remainingAccounts(all_keys)
            .view();

        expect(res.return0).toEqual(new BN(24));
        expect(res.return1).toBe("my name is callee");

        res = await caller.program.methods.doCall4(address_callee, address_callee2, [new BN(1), new BN(2), new BN(3), new BN(4)], "asda")
            .remainingAccounts(all_keys)
            .view();

        expect(res.return0).toEqual(new BN(10));
        expect(res.return1).toBe("x:asda");
    });
});
