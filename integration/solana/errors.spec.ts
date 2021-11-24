import { TransactionError } from '@solana/solidity';
import expect from 'expect';
import { loadContract } from './utils';

describe('Deploy solang contract and test', () => {
    it('errors', async function () {
        this.timeout(50000);

        const { contract } = await loadContract('errors', 'errors.abi')

        let res = await contract.functions.do_revert(false);

        expect(Number(res.result)).toEqual(3124445);

        try {
            res = await contract.functions.do_revert(true, { simulate: true });
        } catch (e) {
            expect(e).toBeInstanceOf(TransactionError);
            if (e instanceof TransactionError) {
                expect(e.message).toBe('Do the revert thing');
                expect(e.computeUnitsUsed).toBe(1050);
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
                expect(e.computeUnitsUsed).toBe(1050);
                expect(e.logs.length).toBeGreaterThan(1);
            }
            return;
        }
    });
});
