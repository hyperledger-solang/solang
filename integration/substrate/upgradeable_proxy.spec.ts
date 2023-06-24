import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';
import { DecodedEvent } from '@polkadot/api-contract/types';
import { AccountId, ContractSelector } from '@polkadot/types/interfaces';

describe('Deploy the upgradable proxy and implementations; expect the upgrade mechanism to work', () => {
    // Helper: Upgrade implementation and execute a constructor that takes no arguments
    async function upgrade_and_constructor(impl: AccountId, constructor: ContractSelector) {
        const params = [impl, constructor];
        const gasLimit = await weight(conn, proxy, 'upgradeToAndCall', params);
        let result: any = await transaction(proxy.tx.upgradeToAndCall({ gasLimit }, ...params), aliceKeypair());

        let events: DecodedEvent[] = result.contractEvents;
        expect(events.length).toEqual(1);
        expect(events[0].event.identifier).toBe("Upgraded");
        expect(events[0].args.map(a => a.toJSON())[0]).toEqual(params[0].toJSON());
    }

    let conn: ApiPromise;
    let alice: KeyringPair;
    let proxy: ContractPromise;
    let counter: ContractPromise;

    before(async function () {
        alice = aliceKeypair();
        conn = await createConnection();

        const proxy_deployment = await deploy(conn, alice, 'UpgradeableProxy.contract', 0n);
        proxy = new ContractPromise(conn, proxy_deployment.abi, proxy_deployment.address);

        // Pretend the proxy contract to be implementation V1
        const implV1_deployment = await deploy(conn, alice, 'UpgradeableImplV1.contract', 0n);
        await upgrade_and_constructor(implV1_deployment.address, implV1_deployment.abi.constructors[0].selector);
        counter = new ContractPromise(conn, implV1_deployment.abi, proxy_deployment.address);
        let count = await query(conn, alice, counter, "count");
        expect(BigInt(count.output?.toString() ?? "")).toStrictEqual(1n);
    });

    after(async function () {
        await conn.disconnect();
    });

    it('Tests implementation and upgrading', async function () {
        // Test implementation V1
        let gasLimit = await weight(conn, counter, 'inc', []);
        await transaction(counter.tx.inc({ gasLimit }), alice);
        await transaction(counter.tx.inc({ gasLimit }), alice);
        let count = await query(conn, alice, counter, "count");
        expect(BigInt(count.output?.toString() ?? "")).toStrictEqual(3n);

        // Upgrade to implementation V2
        const implV2 = await deploy(conn, alice, 'UpgradeableImplV2.contract', 0n);
        await upgrade_and_constructor(implV2.address, implV2.abi.constructors[0].selector);
        counter = new ContractPromise(conn, implV2.abi, proxy.address);

        // Test implementation V2
        count = await query(conn, alice, counter, "count");
        expect(BigInt(count.output?.toString() ?? "")).toStrictEqual(3n);

        gasLimit = await weight(conn, counter, 'dec', []);
        await transaction(counter.tx.dec({ gasLimit }), alice);
        count = await query(conn, alice, counter, "count");
        expect(BigInt(count.output?.toString() ?? "")).toStrictEqual(2n);

        const version = await query(conn, alice, counter, "version");
        expect(version.output?.toString()).toStrictEqual("v2");
    });
});
