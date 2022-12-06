// SPDX-License-Identifier: Apache-2.0

import { TransactionError } from '@solana/solidity';
import expect from 'expect';
import { loadContract } from './setup';

describe('Testing errors', function () {
    this.timeout(500000);

    it('errors', async function () {
        const { contract } = await loadContract('errors');

        let res = await contract.functions.do_revert(false);

        expect(Number(res.result)).toEqual(3124445);

        try {
            res = await contract.functions.do_revert(true, { simulate: true });
        } catch (e) {
            expect(e).toBeInstanceOf(TransactionError);
            if (e instanceof TransactionError) {
                expect(e.message).toBe('Do the revert thing');
                expect(e.computeUnitsUsed).toBeGreaterThan(1400);
                expect(e.computeUnitsUsed).toBeLessThan(1600);
                expect(e.logs.length).toBeGreaterThan(1);
            }
            return;
        }

        try {
            res = await contract.functions.do_revert(true);
        } catch (e) {
            expect(e).toBeInstanceOf(TransactionError);
            if (e instanceof TransactionError) {
                expect(e.message).toBe('Do the revert thing');
                expect(e.computeUnitsUsed).toBeGreaterThan(1400);
                expect(e.computeUnitsUsed).toBeLessThan(1600);
                expect(e.logs.length).toBeGreaterThan(1);
            }
            return;
        }
    });
});
