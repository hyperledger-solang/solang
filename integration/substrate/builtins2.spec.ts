import expect from 'expect';
import { createConnection, deploy, aliceKeypair, gasLimit } from './index';
import { ContractPromise } from '@polkadot/api-contract';

describe('Deploy builtins2 contract and test', () => {
    it('builtins2', async function () {
        this.timeout(50000);

        let conn = await createConnection();
        const alice = aliceKeypair();

        let deployed_contract = await deploy(conn, alice, 'builtins2.contract', BigInt(0));

        let contract = new ContractPromise(conn, deployed_contract.abi, deployed_contract.address);

        let { output: blake2_128 } = await contract.query.hashBlake2128(alice.address, {}, '0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex'));

        expect(blake2_128?.toJSON()).toBe("0x56691483d63cac66c38c168c703c6f13");

        let { output: blake2_256 } = await contract.query.hashBlake2256(alice.address, {}, '0x' + Buffer.from('Call me Ishmael.', 'utf8').toString('hex'));

        expect(blake2_256?.toJSON()).toBe("0x1abd7330c92d835b5084219aedba821c3a599d039d5b66fb5a22ee8e813951a8");

        let { output: _contract_block_number } = await contract.query.blockHeight(alice.address, {});

        let contract_block_number = Number.parseInt(_contract_block_number!.toString());

        let rpc_block_number = await (await conn.query.system.number()).toNumber();

        expect(Math.abs(contract_block_number - rpc_block_number)).toBeLessThanOrEqual(3);

        let { output: gas_left } = await contract.query.burnGas(alice.address, { gasLimit }, 0);
        let gas = BigInt(gas_left!.toString());
        expect(gasLimit).toBeGreaterThan(gas);
        let previous_diff = gasLimit - gas;

        // Gas metering is based on execution time:
        // Expect each call to burn between 10000..1000000 more gas than the previous iteration.
        for (let i = 1; i < 100; i++) {
            let { output: gas_left } = await contract.query.burnGas(alice.address, { gasLimit }, i);
            let gas = BigInt(gas_left!.toString());
            expect(gasLimit).toBeGreaterThan(gas);

            let diff = gasLimit - gas;
            expect(diff).toBeGreaterThan(previous_diff);
            expect(diff - previous_diff).toBeLessThan(1e6);
            expect(diff - previous_diff).toBeGreaterThan(1e4);

            previous_diff = diff;
        }

        conn.disconnect();
    });
});
