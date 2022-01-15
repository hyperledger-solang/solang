import expect from 'expect';
import { loadContract } from './setup';

describe('Deploy solang contract and test', function () {
    this.timeout(500000);

    it('Events', async function () {
        const { contract } = await loadContract('Events', 'Events.abi');

        let res = await contract.functions.getName();

        expect(res.result).toBe("myName");

        await contract.functions.setName('ozan');

        res = await contract.functions.getName();

        expect(res.result).toBe('ozan');

        await contract.functions.setSurname('martin');

        res = await contract.functions.getSurname();

        expect(res.result).toBe('martin');

        res = await contract.functions.getNames();

        expect(res.result[0]).toBe('ozan');
        expect(res.result[1]).toBe('martin');
    });
});
