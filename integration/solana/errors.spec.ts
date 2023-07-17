// SPDX-License-Identifier: Apache-2.0

import { BN } from '@coral-xyz/anchor';
import expect from 'expect';
import { loadContractAndCallConstructor } from './setup';

describe('Testing errors', function () {
    this.timeout(500000);

    it('errors', async function () {
        const { program, storage } = await loadContractAndCallConstructor('errors');

        let res = await program.methods.doRevert(false).view();

        expect(res).toEqual(new BN(3124445));

        try {
            res = await program.methods.doRevert(true).simulate();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain('Program log: Going to revert');
            return;
        }

        try {
            res = await program.methods.doRevert(true)
                .accounts({ dataAccount: storage.publicKey })
                .rpc();
        } catch (e: any) {
            const logs = e.simulationResponse.logs;
            expect(logs).toContain('Program log: Going to revert');
            return;
        }
    });
});
