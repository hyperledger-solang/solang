// SPDX-License-Identifier: Apache-2.0

import { loadContract } from './setup';
import expect from 'expect';
import { BN } from '@coral-xyz/anchor';

describe('Test minimum balance library', function () {
    this.timeout(500000);

    it('minbalance', async function name() {
        const { provider, program } = await loadContract('minbalance');

        const res = await program.methods.test1().view();
        expect(res).toEqual(true);

        for (let i = 50; i <= 150; i += 10) {
            const value = await program.methods.test2(new BN(i)).view();

            const lamports = await provider.connection.getMinimumBalanceForRentExemption(i);

            expect(Number(value)).toBe(lamports);
        }
    });
});