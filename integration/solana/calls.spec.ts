import expect from 'expect';
import { loadProgram } from './utils';
import { Keypair } from '@solana/web3.js';
import fs from 'fs';

describe('Deploy solang contract and test', () => {
    it('external_call', async function () {
        this.timeout(100000);

        const [program, connnection, payerAccount] = await loadProgram();

        const callerStorageKeyPair = Keypair.generate();

        const caller = (await program.deployContract({
            name: 'caller',
            abi: fs.readFileSync('caller.abi', 'utf8'),
            space: 8192,
            storageKeyPair: callerStorageKeyPair,
            constructorArgs: []
        })).contract;

        const calleeStorageKeyPair = Keypair.generate();

        const callee = (await program.deployContract({
            name: 'callee',
            abi: fs.readFileSync('callee.abi', 'utf8'),
            space: 8192,
            storageKeyPair: calleeStorageKeyPair,
            constructorArgs: []
        })).contract;


        const callee2StorageKeyPair = Keypair.generate();
        const callee2 = (await program.deployContract({
            name: 'callee2',
            abi: fs.readFileSync('callee2.abi', 'utf8'),
            space: 8192,
            storageKeyPair: callee2StorageKeyPair,
            constructorArgs: []
        })).contract;

        await callee.functions.set_x(102);

        let res = await callee.functions.get_x({ simulate: true });

        expect(Number(res.result)).toBe(102);

        let address_caller = '0x' + callerStorageKeyPair.publicKey.toBuffer().toString('hex');
        let address_callee = '0x' + calleeStorageKeyPair.publicKey.toBuffer().toString('hex');
        let address_callee2 = '0x' + callee2StorageKeyPair.publicKey.toBuffer().toString('hex');
        console.log("addres: " + address_callee);

        res = await caller.functions.who_am_i({ simulate: true });

        expect(res.result).toBe(address_caller);

        await caller.functions.do_call(address_callee, "13123", {
            writableAccounts: [calleeStorageKeyPair.publicKey],
            accounts: [program.programAccount.publicKey]
        });

        res = await callee.functions.get_x({ simulate: true });

        expect(Number(res.result)).toBe(13123);

        res = await caller.functions.do_call2(address_callee, 20000, {
            simulate: true,
            accounts: [calleeStorageKeyPair.publicKey, program.programAccount.publicKey]
        });

        expect(Number(res.result)).toBe(33123);

        let all_keys = [program.programAccount.publicKey, calleeStorageKeyPair.publicKey, callee2StorageKeyPair.publicKey];

        res = await caller.functions.do_call3(address_callee, address_callee2, ["3", "5", "7", "9"], "yo", { accounts: all_keys });

        expect(Number(res.result[0])).toBe(24);
        expect(res.result[1]).toBe("my name is callee");

        res = await caller.functions.do_call4(address_callee, address_callee2, ["1", "2", "3", "4"], "asda", { accounts: all_keys });

        expect(Number(res.result[0])).toBe(10);
        expect(res.result[1]).toBe("x:asda");
    });
});
