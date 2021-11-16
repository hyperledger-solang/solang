import expect from 'expect';
import { loadContract, load2ndContract } from './utils';
import { Keypair } from '@solana/web3.js';
import fs from 'fs';

describe('Deploy solang contract and test', () => {
    it('external_call', async function () {
        this.timeout(100000);


        const [caller, connection, payerAccount, program] = await loadContract('caller', 'caller.abi');

        const callee = await load2ndContract(connection, program, payerAccount, 'callee', 'callee.abi');


        const callee2 = await load2ndContract(connection, program, payerAccount, 'callee2', 'callee2.abi');

        await callee.functions.set_x(102);

        let res = await callee.functions.get_x({ simulate: true });

        expect(Number(res.result[0])).toBe(102);

        let address_caller = '0x' + caller.storage.toBuffer().toString('hex');
        let address_callee = '0x' + callee.storage.toBuffer().toString('hex');
        let address_callee2 = '0x' + callee2.storage.toBuffer().toString('hex');
        console.log("addres: " + address_callee);

        res = await caller.functions.who_am_i({ simulate: true });

        expect(res.result[0]).toBe(address_caller);

        await caller.functions.do_call(address_callee, "13123", {
            writableAccounts: [callee.storage],
            accounts: [program.publicKey]
        });

        res = await callee.functions.get_x({ simulate: true });

        expect(Number(res.result[0])).toBe(13123);

        res = await caller.functions.do_call2(address_callee, 20000, {
            simulate: true,
            accounts: [callee.storage, program.publicKey]
        });

        expect(Number(res.result[0])).toBe(33123);

        let all_keys = [program.publicKey, callee.storage, callee2.storage];

        res = await caller.functions.do_call3(address_callee, address_callee2, ["3", "5", "7", "9"], "yo", { accounts: all_keys });

        expect(Number(res.result[0])).toBe(24);
        expect(res.result[1]).toBe("my name is callee");

        res = await caller.functions.do_call4(address_callee, address_callee2, ["1", "2", "3", "4"], "asda", { accounts: all_keys });

        expect(Number(res.result[0])).toBe(10);
        expect(res.result[1]).toBe("x:asda");
    });
});
