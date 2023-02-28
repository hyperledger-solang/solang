// SPDX-License-Identifier: Apache-2.0

import expect from 'expect';
import { loadContract } from './setup';

describe('Deploy solang contract and test', function () {
    this.timeout(500000);

    it('Events', async function () {
        const { program, storage } = await loadContract('Events');

        let res = await program.methods.getName()
            .accounts({ dataAccount: storage.publicKey })
            .view();

        expect(res).toBe("myName");

        await program.methods.setName('ozan')
            .accounts({ dataAccount: storage.publicKey })
            .rpc();

        res = await program.methods.getName()
            .accounts({ dataAccount: storage.publicKey })
            .view();

        expect(res).toBe('ozan');

        await program.methods.setSurname('martin')
            .accounts({ dataAccount: storage.publicKey })
            .rpc();

        res = await program.methods.getSurname()
            .accounts({ dataAccount: storage.publicKey })
            .view();

        expect(res).toBe('martin');

        res = await program.methods.getNames()
            .accounts({ dataAccount: storage.publicKey })
            .view();

        expect(res.name).toBe('ozan');
        expect(res.surname).toBe('martin');
    });
});
