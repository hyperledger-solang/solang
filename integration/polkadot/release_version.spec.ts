import { createConnection, deploy, aliceKeypair, debug_buffer } from "./index";
import expect from 'expect';
import { ContractPromise } from "@polkadot/api-contract";

describe('Deploy release_version.sol and test the debug buffer is empty', () => {
    it('removes all debugging', async function () {
        let conn = await createConnection();
        const alice = aliceKeypair();
        let deployed_contract = await deploy(
            conn,
            alice,
            "release.contract",
            BigInt(0)
        );
        let contract = new ContractPromise(
            conn,
            deployed_contract.abi,
            deployed_contract.address
        );

        // The --release flag should remove all debugging features, making the debug buffer empty
        let res = await debug_buffer(conn, contract, `print_then_error`, [20])
        expect(res).toEqual("")

        let res2 = await debug_buffer(conn, contract, `print_then_error`, [0])
        expect(res2).toEqual("")
        conn.disconnect();
    });
});
