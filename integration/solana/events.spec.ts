import expect from 'expect';
import { loadContract } from './utils';

describe('Deploy solang contract and test', () => {
    it('events', async function () {
        this.timeout(50000);

        const [token] = await loadContract('events', 'events.abi');

        await new Promise((resolve) => {
            let first = true;
            let listenId = token.addEventListener(async (ev) => {

                if (first) {
                    expect(Number(ev.args[0])).toEqual(102);
                    expect(ev.args[1]).toEqual(true);
                    expect(ev.args[2]).toEqual('foobar');

                    first = false;
                } else {
                    expect(Number(ev.args[0])).toEqual(500332);
                    expect(ev.args[1]).toEqual("0x41424344");
                    expect(ev.args[2]).toEqual("0xcafe0123");
                }

                await token.removeEventListener(listenId);
                resolve(true);
            });

            token.functions.test();
        });

        let res = await token.functions.test({ simulate: true });

        expect(res.result).toBeNull();
        expect(res.events.length).toBe(2);

        let args = res.events[0].args;

        expect(Number(args[0])).toEqual(102);
        expect(args[1]).toEqual(true);
        expect(args[2]).toEqual('foobar');

        args = res.events[1].args;

        expect(Number(args[0])).toEqual(500332);
        expect(args[1]).toEqual("0x41424344");
        expect(args[2]).toEqual("0xcafe0123");
    });
});
