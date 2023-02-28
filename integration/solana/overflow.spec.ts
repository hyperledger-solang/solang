// SPDX-License-Identifier: Apache-2.0

import expect from 'expect';
import { loadContract } from './setup';
import { BN } from '@project-serum/anchor';

describe('Testing math overflow', function () {
    this.timeout(500000);

    it('overflow', async function () {
        let { program } = await loadContract('overflow');

        let res = await program.methods.addu32(new BN(1), new BN(3)).view();
        expect(res).toEqual(4);

        await expect(program.methods.addu32(new BN(2147483648), new BN(2147483648)).view()).rejects.toThrow();

        res = await program.methods.subu32(new BN(7), new BN(3)).view();
        expect(res).toEqual(4);

        await expect(program.methods.subu32(new BN(2147483640), new BN(2147483648)).view()).rejects.toThrow();

        res = await program.methods.mulu32(new BN(7), new BN(3)).view();
        expect(res).toEqual(21);

        await expect(program.methods.mulu32(new BN(2147483640), new BN(2147483648)).view()).rejects.toThrow();

        res = await program.methods.powu32(new BN(7), new BN(3)).view();
        expect(res).toEqual(343);

        await expect(program.methods.powu32(new BN(2147483640), new BN(2147483648)).view()).rejects.toThrow();
    });
});
