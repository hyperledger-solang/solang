import expect from 'expect';
import { createConnection, deploy, aliceKeypair, query, weight, transaction } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';

describe('Deploy the caller_is_root contract and test it', () => {
    let conn: ApiPromise;
    let contract: ContractPromise;
    let alice: KeyringPair;

    before(async function () {
        conn = await createConnection();
        alice = aliceKeypair();
        const instance = await deploy(conn, alice, 'CallerIsRoot.contract', 0n);
        contract = new ContractPromise(conn, instance.abi, instance.address);
    });

    after(async function () {
        await conn.disconnect();
    });

    it('is correct on a root caller', async function () {
        // Without sudo the caller should not be root
        let gasLimit = await weight(conn, contract, "covert");
        await transaction(contract.tx.covert({ gasLimit }), alice);

        // Calling `covert` as non-root sets the balance to 1
        let balance = await query(conn, alice, contract, "balance", []);
        expect(BigInt(balance.output?.toString() ?? "")).toStrictEqual(1n);


        // Alice has sudo rights on --dev nodes
        gasLimit = await weight(conn, contract, "covert");
        await transaction(conn.tx.sudo.sudo(contract.tx.covert({ gasLimit })), alice);

        // Calling `covert` as root sets the balance to 0xdeadbeef
        balance = await query(conn, alice, contract, "balance", []);
        expect(BigInt(balance.output?.toString() ?? "")).toStrictEqual(0xdeadbeefn);
    });
});