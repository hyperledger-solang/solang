import expect from 'expect';
import { createConnection, deploy, aliceKeypair, query, weight, transaction } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise, Keyring } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';
import { DecodedEvent } from '@polkadot/api-contract/types';

describe('Deploy the caller_is_root contract and test it', () => {
    let conn: ApiPromise;
    let oracle: ContractPromise;
    let alice: KeyringPair;

    before(async function () {
        alice = aliceKeypair();
        conn = await createConnection();
        const contract = await deploy(conn, alice, 'CallerIsRoot.contract', 0n);
        oracle = new ContractPromise(conn, contract.abi, contract.address);
    });

    after(async function () {
        await conn.disconnect();
    });

    it('is correct on a root caller', async function () {
        const value = 0xdeadbeefn;


        // The balance should default to 0
        let balance = await query(conn, alice, oracle, "balance", []);
        expect(BigInt(balance.output?.toString() ?? "")).toStrictEqual(0n);


        // Without sudo the caller should not be root
        let gasLimit = await weight(conn, oracle, "admin");
        let tx = oracle.tx.admin({ gasLimit });
        await transaction(tx, alice);

        balance = await query(conn, alice, oracle, "balance", []);
        expect(BigInt(balance.output?.toString() ?? "")).toStrictEqual(0n);


        // Alice has sudo rights on --dev nodes
        gasLimit = await weight(conn, oracle, "admin");
        tx = oracle.tx.admin({ gasLimit });
        let res0: any = await transaction(conn.tx.sudo.sudo(tx), alice);
        const events: DecodedEvent[] = res0.contractEvents;
        console.log("events: ", events);

        balance = await query(conn, alice, oracle, "balance", []);
        expect(BigInt(balance.output?.toString() ?? "")).toStrictEqual(value);
    });
});