import expect from 'expect';
import { createConnection, deploy, aliceKeypair, query, debug_buffer, weight, transaction, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';
import { U8aFixed } from '@polkadot/types';

describe('Deploy the SetCodeCounter contracts and test for the upgrade to work', () => {
    let conn: ApiPromise;
    let counter: ContractPromise;
    let hashes: [U8aFixed, U8aFixed];
    let alice: KeyringPair;

    before(async function () {
        alice = aliceKeypair();
        conn = await createConnection();

        const counterV1 = await deploy(conn, alice, 'SetCodeCounterV1.contract', 0n, 1336n);
        const counterV2 = await deploy(conn, alice, 'SetCodeCounterV2.contract', 0n, 0n);
        hashes = [counterV1.abi.info.source.wasmHash, counterV2.abi.info.source.wasmHash];
        counter = new ContractPromise(conn, counterV1.abi, counterV1.address);
    });

    after(async function () {
        await conn.disconnect();
    });

    it('can switch out implementation using set_code_hash', async function () {
        // Code hash should be V1, expect to increment
        let gasLimit = await weight(conn, counter, 'inc', []);
        await transaction(counter.tx.inc({ gasLimit }), alice);
        let count = await query(conn, alice, counter, "count");
        expect(BigInt(count.output?.toString() ?? "")).toStrictEqual(1337n);

        // Switching to V2
        gasLimit = await weight(conn, counter, 'set_code', [hashes[1]]);
        await transaction(counter.tx.setCode({ gasLimit }, hashes[1]), alice);

        // Code hash should be V2, expect to decrement
        gasLimit = await weight(conn, counter, 'inc', []);
        await transaction(counter.tx.inc({ gasLimit }), alice);
        count = await query(conn, alice, counter, "count");
        expect(BigInt(count.output?.toString() ?? "")).toStrictEqual(1336n);

        // Switching to V1
        gasLimit = await weight(conn, counter, 'set_code', [hashes[0]]);
        await transaction(counter.tx.setCode({ gasLimit }, hashes[0]), alice);

        // Code hash should be V1, expect to increment
        gasLimit = await weight(conn, counter, 'inc', []);
        await transaction(counter.tx.inc({ gasLimit }), alice);
        count = await query(conn, alice, counter, "count");
        expect(BigInt(count.output?.toString() ?? "")).toStrictEqual(1337n);
    });

});
