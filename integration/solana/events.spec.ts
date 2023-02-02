// SPDX-License-Identifier: Apache-2.0

import expect from 'expect';
import { loadContract } from './setup';

describe('Test events', function () {
    this.timeout(500000);

    it('events', async function () {
        const { program, storage } = await loadContract('MyContractEvents');

        const res = await program.methods.test()
            .accounts({ dataAccount: storage.publicKey })
            .simulate();

        const event1 = res.events[0];

        expect(event1.name).toEqual('First');
        expect(event1.data.a).toEqual(102);
        expect(event1.data.b).toEqual(true);
        expect(event1.data.c).toEqual('foobar');

        const event2 = res.events[1];

        expect(event2.name).toEqual('Second');
        expect(event2.data.a).toEqual(500332);
        expect(event2.data.b).toEqual('ABCD');
        expect(event2.data.c).toEqual('CAFE0123');
    });
});
