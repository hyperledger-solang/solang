import { Keypair } from '@solana/web3.js';
import expect from 'expect';
import { createProgramAddress, establishConnection } from './index';

describe('Deploy solang contract and test', () => {
    it('create_contract', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        let creator = await conn.loadProgram("bundle.so", "creator.abi");

        // call the constructor
        await creator.call_constructor(conn, 'creator', []);

        let seed = await createProgramAddress(creator.get_program_key());

        console.log("now create child: " + seed.address.toString());

        await creator.call_function(conn, "create_child", [], [creator.get_program_key()], [seed], [creator.get_storage_keypair()]);
    });
});
