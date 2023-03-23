import { createConnection, deploy, aliceKeypair, debug_buffer } from "./index";
import expect from 'expect';
import { ContractPromise } from "@polkadot/api-contract";

describe('Deploy runtime_errors.sol and test the debug buffer', () => {
    it('logs errors to the debug buffer', async function () {
        let conn = await createConnection();
        const alice = aliceKeypair();
        let deployed_contract = await deploy(
            conn,
            alice,
            "RuntimeErrors.contract",
            BigInt(0)
        );
        let contract = new ContractPromise(
            conn,
            deployed_contract.abi,
            deployed_contract.address
        );
        let child_contract = await deploy(conn, alice, 'child_runtime_errors.contract', BigInt(0));

        let res = await debug_buffer(conn, contract, `get_storage_bytes`, [])
        expect(res).toContain(`runtime_error: storage array index out of bounds in runtime_errors.sol:46:19-23,`)

        let res1 = await debug_buffer(conn, contract, `transfer_abort`, [])
        expect(res1).toContain(`runtime_error: value transfer failure in runtime_errors.sol:53:29-31,`)

        let res2 = await debug_buffer(conn, contract, `pop_empty_storage`, [])
        expect(res2).toContain(`runtime_error: pop from empty storage array in runtime_errors.sol:58:13-16,`)

        let res3 = await debug_buffer(conn, contract, `call_ext`, [child_contract.address])
        expect(res3).toContain(`runtime_error: external call failed in runtime_errors.sol:63:9-24,`)

        let res4 = await debug_buffer(conn, contract, `create_child`);
        expect(res4).toContain(`runtime_error: contract creation failed in runtime_errors.sol:68:13-39,`)

        let res5 = await debug_buffer(conn, contract, `set_storage_bytes`, [])
        expect(res5).toContain(`runtime_error: storage index out of bounds in runtime_errors.sol:39:11-12,`)

        let res6 = await debug_buffer(conn, contract, `dont_pay_me`, [], 1);
        expect(res6).toContain(`runtime_error: non payable function dont_pay_me received value,`)

        let res7 = await debug_buffer(conn, contract, `assert_test`, [9], 0);
        expect(res7).toContain(`runtime_error: assert failure in runtime_errors.sol:32:16-24,`)

        let res8 = await debug_buffer(conn, contract, `i_will_revert`, [], 0);
        expect(res8).toContain(`runtime_error: revert encountered in runtime_errors.sol:77:9-15,`)

        let res9 = await debug_buffer(conn, contract, `write_integer_failure`, [1], 0);
        expect(res9).toContain(`runtime_error: integer too large to write in buffer in runtime_errors.sol:82:18-31,`)

        let res10 = await debug_buffer(conn, contract, `write_bytes_failure`, [9], 0);
        expect(res10).toContain(`runtime_error: data does not fit into buffer in runtime_errors.sol:88:18-28,`)

        let res11 = await debug_buffer(conn, contract, `read_integer_failure`, [2], 0);
        expect(res11).toContain(`runtime_error: read integer out of bounds in runtime_errors.sol:93:18-30,`)

        let res12 = await debug_buffer(conn, contract, `trunc_failure`, [BigInt(`999999999999999999999999`)], 0);
        expect(res12).toContain(`runtime_error: truncated type overflows in runtime_errors.sol:98:37-42,`)

        let res13 = await debug_buffer(conn, contract, `out_of_bounds`, [19], 0);
        expect(res13).toContain(`runtime_error: array index out of bounds in runtime_errors.sol:104:16-21,`)

        let res14 = await debug_buffer(conn, contract, `invalid_instruction`, [], 0);
        expect(res14).toContain(`runtime_error: reached invalid instruction in runtime_errors.sol:109:13-22,`)

        let res15 = await debug_buffer(conn, contract, `byte_cast_failure`, [33], 0);
        expect(res15).toContain(`runtime_error: bytes cast error in runtime_errors.sol:115:23-40,`)

        conn.disconnect();
    });
});
