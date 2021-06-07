import { Keypair } from '@solana/web3.js';
import expect from 'expect';
import { establishConnection } from './index';

describe('Deploy solang contract and test', () => {
    it('create_contract', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        let creator = await conn.loadProgram("bundle.so", "creator.abi");

        // call the constructor
        await creator.call_constructor(conn, 'creator', []);

        console.log("now create child");

        let child = await conn.createStorageAccount(creator.get_program_key(), 1024);

        await creator.call_function(conn, "create_child", [], [child.publicKey, creator.get_program_key()]);
    });
});
