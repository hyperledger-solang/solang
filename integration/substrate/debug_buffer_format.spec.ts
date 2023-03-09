import { createConnection, deploy, aliceKeypair, debug_buffer } from "./index";
import expect from 'expect';
import { ContractPromise } from "@polkadot/api-contract";

describe('Deploy debug_buffer_format.sol and test the debug buffer formatting', () => {
    it('formats the debug buffer', async function () {

        let conn = await createConnection();
        const alice = aliceKeypair();


        let deployed_contract = await deploy(
            conn,
            alice,
            "DebugBuffer.contract",
            BigInt(0)
        );

        let contract = new ContractPromise(
            conn,
            deployed_contract.abi,
            deployed_contract.address
        );



        let res = await debug_buffer(conn, contract, "multiple_prints", [])
        expect(res).toEqual(`print: Hello!,
call: seal_debug_message=0,
print: I call seal_debug_message under the hood!,
call: seal_debug_message=0,
`)


        let res1 = await debug_buffer(conn, contract, "multiple_prints_then_revert", [])
        expect(res1).toEqual(`print: Hello!,
call: seal_debug_message=0,
print: I call seal_debug_message under the hood!,
call: seal_debug_message=0,
runtime_error: sesa!!! revert encountered in debug_buffer_format.sol:10:9-15,
call: seal_debug_message=0,
`)



        conn.disconnect();
    });
});
