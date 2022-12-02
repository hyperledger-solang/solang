import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';

describe('Test cross contract calls between ink and solidity', () => {
    let conn: ApiPromise;
    let alice: KeyringPair;

    let ink_contract: ContractPromise;

    let sol_contract: ContractPromise;
    let inkee_echo = [1, 2, 3, 4];

    before(async function () {
        conn = await createConnection();
        alice = aliceKeypair();

        let ink_deployment = await deploy(conn, alice, 'ink/caller/target/ink/caller.contract', 0n);
        ink_contract = new ContractPromise(conn, ink_deployment.abi, ink_deployment.address);

        let sol_deployment = await deploy(conn, alice, 'Inkee.contract', 0n);
        sol_contract = new ContractPromise(conn, sol_deployment.abi, sol_deployment.address);
    });

    it('calls solidity from ink', async function () {
        this.timeout(50000);

        async function proxy(goes_in: number) {
            const comes_out = await query(conn, alice, ink_contract, "u32_proxy", [sol_contract.address, inkee_echo, goes_in, null, null]);
            expect(comes_out.output?.toJSON()).toEqual({ "ok": goes_in });
        }

        await proxy(0);
        await proxy(1);
        await proxy(1337);
        await proxy(0xffffffff);
    });

    after(async function () {
        await conn.disconnect();
    });
});
