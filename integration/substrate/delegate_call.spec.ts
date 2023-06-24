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


describe('Deploy the upgradable proxy and implementations; expect the proxy call and upgrade mechanism to work', () => {
    let conn: ApiPromise;
    let proxy: ContractPromise;
    let implV1: ContractPromise;
    let implV2: ContractPromise;
    let alice: KeyringPair;

    before(async function () {
        alice = aliceKeypair();
        conn = await createConnection();

        const implV1_deployment = await deploy(conn, alice, 'UpgradeableImplV1.contract', 0n);
        implV1 = new ContractPromise(conn, implV1_deployment.abi, implV1_deployment.address);

        const implV2_deployment = await deploy(conn, alice, 'UpgradeableImplV2.contract', 0n);
        implV2 = new ContractPromise(conn, implV2_deployment.abi, implV2_deployment.address);

        //const constructor = implV1.abi.constructors[0].selector.toJSON();
        const proxy_deployment = await deploy(conn, alice, 'UpgradeableProxy.contract', 0n, implV1.address);
        console.log(proxy_deployment);
        proxy = new ContractPromise(conn, implV1_deployment.abi, proxy_deployment.address);
    });

    after(async function () {
        await conn.disconnect();
    });

    it('Works', async function () {
        const gasLimit = await weight(conn, proxy, 'inc', []);
        await transaction(proxy.tx.inc({ gasLimit }), alice);
        await transaction(proxy.tx.inc({ gasLimit }), alice);
        let count = await query(conn, alice, proxy, "count");
        expect(BigInt(count.output?.toString() ?? "")).toStrictEqual(2n);
    });
});
