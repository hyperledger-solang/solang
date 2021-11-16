import { SimulationError } from '@solana/solidity';
import expect from 'expect';
import { loadContract } from './utils';

describe('Deploy solang contract and test', () => {
    it('errors', async function () {
        this.timeout(50000);

        const [token] = await loadContract('errors', 'errors.abi')

        let res = await token.functions.do_revert(false);

        expect(Number(res.result[0])).toEqual(3124445);

        try {
            res = await token.functions.do_revert(true, { simulate: true });
        } catch (e) {
            expect(e).toBeInstanceOf(SimulationError);
            if (e instanceof SimulationError) {
                expect(e.message).toBe('Do the revert thing');
                expect(e.computeUnitsUsed).toBe(1050);
                expect(e.logs.length).toBeGreaterThan(1);
            }
            return;
        }

        try {
            res = await token.functions.do_revert(true);
        } catch (e) {
            expect(e).toBeInstanceOf(SimulationError);
            if (e instanceof SimulationError) {
                expect(e.message).toBe('Do the revert thing');
                expect(e.computeUnitsUsed).toBe(1050);
                expect(e.logs.length).toBeGreaterThan(1);
            }
            return;
        }
    });
});
