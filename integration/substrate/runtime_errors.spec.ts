import { createConnection, deploy, aliceKeypair, debug_buffer } from "./index";
import expect from 'expect';
import { ContractPromise } from "@polkadot/api-contract";
import path = require("path")

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

    let child_contract = await deploy(conn, alice, 'child.contract', BigInt(0));

    let res = await debug_buffer(conn, contract, "get_storage_bytes", [])
    expect(res).toEqual("storage array index out of bounds in runtime_errors.sol:46:18")

    let res1 = await debug_buffer(conn, contract, "transfer_abort", [])
    expect(res1).toEqual("value transfer failure in runtime_errors.sol:53:28")


    let res2 = await debug_buffer(conn, contract, "pop_empty_storage", [])
    expect(res2).toEqual("pop from empty storage array in runtime_errors.sol:58:12")


    let res3 = await debug_buffer(conn, contract, "call_ext", [child_contract.address])
    expect(res3).toEqual("external call failed in runtime_errors.sol:63:8")


    let res4 = await debug_buffer(conn, contract, "create_child");
    expect(res4).toEqual("contract creation failed in runtime_errors.sol:68:12")


    let res5 = await debug_buffer(conn, contract, "set_storage_bytes", [])
    expect(res5).toEqual("storage index out of bounds in runtime_errors.sol:39:10")


    let res6 = await debug_buffer(conn, contract, "dont_pay_me", [], 1);
    expect(res6).toEqual("non payable function dont_pay_me received value")


    let res7 = await debug_buffer(conn, contract, "assert_test", [9], 0);
    expect(res7).toEqual("assert failure in runtime_errors.sol:32:15")


    let res8 = await debug_buffer(conn, contract, "i_will_revert", [], 0);
    expect(res8).toEqual("revert encountered in runtime_errors.sol:77:8")


    let res9 = await debug_buffer(conn, contract, "write_integer_failure", [1], 0);
    expect(res9).toEqual("integer too large to write in buffer in runtime_errors.sol:82:17")


    let res10 = await debug_buffer(conn, contract, "write_bytes_failure", [9], 0);
    expect(res10).toEqual("data does not fit into buffer in runtime_errors.sol:88:17")


    let res11 = await debug_buffer(conn, contract, "read_integer_failure", [2], 0);
    expect(res11).toEqual("read integer out of bounds in runtime_errors.sol:93:17")


    let res12 = await debug_buffer(conn, contract, "trunc_failure", [BigInt("999999999999999999999999")], 0);
    expect(res12).toEqual("truncated type overflows in runtime_errors.sol:98:36")


    let res13 = await debug_buffer(conn, contract, "out_of_bounds", [19], 0);
    expect(res13).toEqual("array index out of bounds in runtime_errors.sol:104:15")


    let res14 = await debug_buffer(conn, contract, "invalid_instruction", [], 0);
    expect(res14).toEqual("reached invalid instruction in runtime_errors.sol:109:12")


    let res15 = await debug_buffer(conn, contract, "byte_cast_failure", [33], 0);
    expect(res15).toEqual("bytes cast error in runtime_errors.sol:115:22")

    conn.disconnect();
  });
});
