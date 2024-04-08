// SPDX-License-Identifier: Apache-2.0

import expect from 'expect';
import { createConnection, deploy, aliceKeypair, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';

describe('Deploy the is_contract oracle and test it on contract an non-contract addresses', () => {
    let conn: ApiPromise;
    let oracle: ContractPromise;
    let alice: KeyringPair;

    before(async function () {
        alice = aliceKeypair();
        conn = await createConnection();
        const contract = await deploy(conn, alice, 'IsContractOracle.contract', 0n);
        oracle = new ContractPromise(conn, contract.abi, contract.address);
    });

    after(async function () {
        await conn.disconnect();
    });

    it('is correct on a contract address', async function () {
        const answer = await query(conn, alice, oracle, "contract_oracle", [oracle.address]);
        expect(answer.output?.toJSON()).toStrictEqual(true);
    });

    it('is correct on a non-contract address', async function () {
        const answer = await query(conn, alice, oracle, "contract_oracle", [alice.address]);
        expect(answer.output?.toJSON()).toStrictEqual(false);
    });
});
