import expect from 'expect';
import { loadContract } from './utils';

describe('Deploy solang contract and test', () => {
    it('Events', async function () {
        this.timeout(50000);

        const [token] = await loadContract('Events', 'Events.abi');

        let res = await token.functions.getName();

        expect(res.result).toBe("myName");

        await token.functions.setName('ozan');

        res = await token.functions.getName();

        expect(res.result).toBe('ozan');

        await token.functions.setSurname('martin');

        res = await token.functions.getSurname();

        expect(res.result).toBe('martin');

        res = await token.functions.getNames();

        expect(res.result[0]).toBe('ozan');
        expect(res.result[1]).toBe('martin');
    });
});
