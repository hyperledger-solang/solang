import { createConnection, deploy, aliceKeypair, query, debug_buffer, weight, transaction, daveKeypair, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';
import expect from 'expect';

describe('Test that the constructor can not be reached from the call function', () => {
    let conn: ApiPromise;
    let contract: ContractPromise;
    let caller: ContractPromise;
    let alice: KeyringPair;

    before(async function () {
        alice = aliceKeypair();
        conn = await createConnection();

        const contract_deployment = await deploy(conn, alice, 'ConstructorDispatch.contract', 0n);
        contract = new ContractPromise(conn, contract_deployment.abi, contract_deployment.address);
        const caller_deployment = await deploy(conn, alice, 'HappyCaller.contract', 0n);
        caller = new ContractPromise(conn, caller_deployment.abi, caller_deployment.address);

    });

    after(async function () {
        await conn.disconnect();
    });

    it('Should fail to overwrite the admin account of the target contract', async function () {
        // "Call" the constructor
        const input = contract.abi.constructors[0].selector;
        let gasLimit = await weight(conn, caller, 'call', [contract.address, input]);
        await transaction(caller.tx.call({ gasLimit }, contract.address, input), alice);

        // Alice must still be admin
        let admin = await query(conn, alice, contract, "boss");
        expect(admin.output?.toString()).toStrictEqual(alice.address.toString());
    });

});
