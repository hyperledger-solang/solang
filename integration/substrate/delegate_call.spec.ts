// Tests against the tornado cash core contracts.
// The tornado contracts used here contain minor mechanical changes to work fine on Substrate.
// The ZK-SNARK setup is the same as ETH Tornado on mainnet.
// On the node, the MiMC sponge hash (available as EVM bytecode) and bn128 curve operations
// (precompiled contracts on Ethereum) are expected to be implemented as chain extensions.

import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, daveKeypair, debug_buffer, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';


describe('Deploy the delegator and the delegatee contracts; test the delegatecall to work correct', () => {
    let conn: ApiPromise;
    let delegatee: ContractPromise;
    let delegator: ContractPromise;
    let alice: KeyringPair;
    let dave: KeyringPair;

    before(async function () {
        alice = aliceKeypair();
        dave = daveKeypair();
        conn = await createConnection();

        const delegator_contract = await deploy(conn, alice, 'Delegator.contract', 0n);
        delegator = new ContractPromise(conn, delegator_contract.abi, delegator_contract.address);

        const delegatee_contract = await deploy(conn, alice, 'Delegatee.contract', 0n);
        delegatee = new ContractPromise(conn, delegatee_contract.abi, delegatee_contract.address);

        // Set delegatee storage to default values and alice address
        const gasLimit = await weight(conn, delegatee, 'setVars', [0n]);
        await transaction(delegatee.tx.setVars({ gasLimit }, [0n]), alice);
    });

    after(async function () {
        await conn.disconnect();
    });

    it('Executes the delegatee in the contex of the delegator', async function () {
        const value = 1000000n;
        const arg = 123456789n;
        const parameters = [delegatee.address, arg];

        const gasLimit = await weight(conn, delegator, 'setVars', parameters);
        await transaction(delegator.tx.setVars({ gasLimit, value }, ...parameters), dave);

        // Storage of the delegatee must not change
        let num = await query(conn, alice, delegatee, "num");
        expect(BigInt(num.output?.toString() ?? "")).toStrictEqual(0n);
        let balance = await query(conn, alice, delegatee, "value");
        expect(BigInt(balance.output?.toString() ?? "")).toStrictEqual(0n);
        let sender = await query(conn, alice, delegatee, "sender");
        expect(sender.output?.toJSON()).toStrictEqual(alice.address);

        // Storage of the delegator must have changed
        num = await query(conn, alice, delegator, "num");
        expect(BigInt(num.output?.toString() ?? "")).toStrictEqual(arg);
        balance = await query(conn, alice, delegator, "value");
        expect(BigInt(balance.output?.toString() ?? "")).toStrictEqual(value);
        sender = await query(conn, alice, delegator, "sender");
        expect(sender.output?.toJSON()).toStrictEqual(dave.address);
    });
});
