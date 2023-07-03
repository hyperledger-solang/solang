import expect from "expect";
import { aliceKeypair, createConnection, deploy, weight, transaction, query } from "./index";
import { ContractPromise } from "@polkadot/api-contract";
import { DecodedEvent } from "@polkadot/api-contract/types";
import { ApiPromise } from "@polkadot/api";

describe('Deploy mytoken contract and test', () => {
    let conn: ApiPromise

    beforeEach(async function () {
        conn = await createConnection();
    });

    afterEach(async function () {
        await conn.disconnect();
    });

    it('mytoken', async function () {
        this.timeout(100000);

        const alice = aliceKeypair();

        let deployed_contract = await deploy(conn, alice, 'mytoken.contract', BigInt(0));
        let contract = new ContractPromise(conn, deployed_contract.abi, deployed_contract.address);

        let res = await query(conn, alice, contract, "test", [alice.address, true]);
        expect(res.output?.toJSON()).toEqual(alice.address);

        res = await query(conn, alice, contract, "test", [alice.address, false]);
        expect(res.output?.toJSON()).toEqual(alice.address);
    });

    it('mytokenEvent', async function () {
        this.timeout(100000);

        const alice = aliceKeypair();

        let deployed_contract = await deploy(conn, alice, 'mytokenEvent.contract', BigInt(0));
        let contract = new ContractPromise(conn, deployed_contract.abi, deployed_contract.address);
        let gasLimit = await weight(conn, contract, "test");
        let tx = contract.tx.test({ gasLimit });
        let res0: any = await transaction(tx, alice);

        let events: DecodedEvent[] = res0.contractEvents;

        expect(events.length).toEqual(1);

        expect(events[0].event.identifier).toBe("Debugging");
        expect(events[0].args.map(a => a.toJSON())).toEqual([alice.address]);
    });
});