
import expect from 'expect';
import { createConnection, deploy, aliceKeypair, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';

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

    it('is correct on a non-root caller', async function () {
        const answer = await query(conn, alice, oracle, "is_root", []);
        expect(answer.output?.toJSON()).toStrictEqual(false);
    });
});