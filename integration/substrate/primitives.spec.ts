import expect from 'expect';
import { createConnection, deploy, aliceKeypair, daveKeypair, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';

describe('Deploy primitives contract and test', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('primitives', async function () {
        this.timeout(100000);

        const alice = aliceKeypair();
        const dave = daveKeypair();

        let deployed_contract = await deploy(conn, alice, 'primitives.contract', BigInt(0));

        let contract = new ContractPromise(conn, deployed_contract.abi, deployed_contract.address);

        // TEST Basic enums
        // in ethereum, an enum is described as an uint8 so can't use the enum
        // names programmatically. 0 = add, 1 = sub, 2 = mul, 3 = div, 4 = mod, 5 = pow, 6 = shl, 7 = shr
        let res = await query(conn, alice, contract, "isMul", [2]);
        expect(res.output?.toJSON()).toEqual(true);

        res = await query(conn, alice, contract, "returnDiv");
        expect(res.output?.toJSON()).toEqual("div");

        // TEST uint and int types, and arithmetic/bitwise ops
        res = await query(conn, alice, contract, "opI64", [0, 1000, 4100]);
        expect(res.output?.toJSON()).toEqual(5100);
        res = await query(conn, alice, contract, "opI64", [1, 1000, 4100]);
        expect(res.output?.toJSON()).toEqual(-3100);
        res = await query(conn, alice, contract, "opI64", [2, 1000, 4100]);
        expect(res.output?.toJSON()).toEqual(4100000);
        res = await query(conn, alice, contract, "opI64", [3, 1000, 10]);
        expect(res.output?.toJSON()).toEqual(100);
        res = await query(conn, alice, contract, "opI64", [4, 1000, 99]);
        expect(res.output?.toJSON()).toEqual(10);
        res = await query(conn, alice, contract, "opI64", [6, - 1000, 8]);
        expect(res.output?.toJSON()).toEqual(-256000);
        res = await query(conn, alice, contract, "opI64", [7, - 1000, 8]);
        expect(res.output?.toJSON()).toEqual(-4);


        res = await query(conn, alice, contract, "opU64", [0, 1000, 4100]);
        expect(res.output?.toJSON()).toEqual(5100);
        res = await query(conn, alice, contract, "opU64", [1, 1000, 4100]);
        expect(res.output?.toString()).toEqual("18446744073709548516"); // (2^64)-18446744073709548516 = 3100
        res = await query(conn, alice, contract, "opU64", [2, 123456789, 123456789]);
        expect(res.output?.toString()).toEqual("15241578750190521");
        res = await query(conn, alice, contract, "opU64", [3, 123456789, 100]);
        expect(res.output?.toJSON()).toEqual(1234567);
        res = await query(conn, alice, contract, "opU64", [4, 123456789, 100]);
        expect(res.output?.toJSON()).toEqual(89);
        res = await query(conn, alice, contract, "opU64", [5, 3, 7]);
        expect(res.output?.toJSON()).toEqual(2187);
        res = await query(conn, alice, contract, "opI64", [6, 1000, 8]);
        expect(res.output?.toJSON()).toEqual(256000);
        res = await query(conn, alice, contract, "opI64", [7, 1000, 8]);
        expect(res.output?.toJSON()).toEqual(3);

        // // now for 256 bit operations
        res = await query(conn, alice, contract, "opI256", [0, 1000, 4100]);
        expect(res.output?.toJSON()).toEqual(5100);
        res = await query(conn, alice, contract, "opI256", [1, 1000, 4100]);
        expect(res.output?.toJSON()).toEqual(-3100);
        res = await query(conn, alice, contract, "opI256", [2, 1000, 4100]);
        expect(res.output?.toJSON()).toEqual(4100000);
        res = await query(conn, alice, contract, "opI256", [3, 1000, 10]);
        expect(res.output?.toJSON()).toEqual(100);
        res = await query(conn, alice, contract, "opI256", [4, 1000, 99]);
        expect(res.output?.toJSON()).toEqual(10);
        res = await query(conn, alice, contract, "opI256", [6, - 10000000000000, 8]);
        expect(res.output?.toJSON()).toEqual(-2560000000000000);
        res = await query(conn, alice, contract, "opI256", [7, - 10000000000000, 8]);
        expect(res.output?.toJSON()).toEqual(-39062500000);

        res = await query(conn, alice, contract, "opU256", [0, 1000, 4100]);
        expect(res.output?.toJSON()).toEqual(5100);
        res = await query(conn, alice, contract, "opU256", [1, 1000, 4100]);
        expect(res.output?.toString()).toEqual('115792089237316195423570985008687907853269984665640564039457584007913129636836'); // (2^64)-18446744073709548516 = 3100
        res = await query(conn, alice, contract, "opU256", [2, 123456789, 123456789]);
        expect(res.output?.toString()).toEqual('15241578750190521');
        res = await query(conn, alice, contract, "opU256", [3, 123456789, 100]);
        expect(res.output?.toJSON()).toEqual(1234567);
        res = await query(conn, alice, contract, "opU256", [4, 123456789, 100]);
        expect(res.output?.toJSON()).toEqual(89);
        res = await query(conn, alice, contract, "opU256", [5, 123456789, 9]);
        expect(res.output?.toString()).toEqual('6662462759719942007440037531362779472290810125440036903063319585255179509');
        res = await query(conn, alice, contract, "opI256", [6, 10000000000000, 8]);
        expect(res.output?.toJSON()).toEqual(2560000000000000);
        res = await query(conn, alice, contract, "opI256", [7, 10000000000000, 8]);
        expect(res.output?.toJSON()).toEqual(39062500000);

        // TEST bytesN
        res = await query(conn, alice, contract, "returnU86");
        expect(res.output?.toJSON()).toEqual('0x414243444546');

        // TEST bytes5
        res = await query(conn, alice, contract, "opU85Shift", [6, '0xdeadcafe59', 8]);
        expect(res.output?.toJSON()).toEqual('0xadcafe5900');
        res = await query(conn, alice, contract, "opU85Shift", [7, '0xdeadcafe59', 8]);
        expect(res.output?.toJSON()).toEqual('0x00deadcafe');
        res = await query(conn, alice, contract, "opU85", [8, '0xdeadcafe59', '0x0000000006']);
        expect(res.output?.toJSON()).toEqual('0xdeadcafe5f');
        res = await query(conn, alice, contract, "opU85", [9, '0xdeadcafe59', '0x00000000ff']);
        expect(res.output?.toJSON()).toEqual('0x0000000059');
        res = await query(conn, alice, contract, "opU85", [10, '0xdeadcafe59', '0x00000000ff']);
        expect(res.output?.toJSON()).toEqual('0xdeadcafea6');

        // TEST bytes14
        res = await query(conn, alice, contract, "opU814Shift", [6, '0xdeadcafe123456789abcdefbeef7', 9]);
        expect(res.output?.toJSON()).toEqual('0x5b95fc2468acf13579bdf7ddee00');
        res = await query(conn, alice, contract, "opU814Shift", [7, '0xdeadcafe123456789abcdefbeef7', 9]);
        expect(res.output?.toJSON()).toEqual('0x006f56e57f091a2b3c4d5e6f7df7');
        res = await query(conn, alice, contract, "opU814", [8, '0xdeadcafe123456789abcdefbeef7', '0x0000060000000000000000000000']);
        expect(res.output?.toJSON()).toEqual('0xdeadcefe123456789abcdefbeef7');
        res = await query(conn, alice, contract, "opU814", [9, '0xdeadcafe123456789abcdefbeef7', '0x000000000000000000ff00000000']);
        expect(res.output?.toJSON()).toEqual('0x000000000000000000bc00000000');
        res = await query(conn, alice, contract, "opU814", [10, '0xdeadcafe123456789abcdefbeef7', '0xff00000000000000000000000000']);
        expect(res.output?.toJSON()).toEqual('0x21adcafe123456789abcdefbeef7');

        // TEST address type.
        const default_account = '5GBWmgdFAMqm8ZgAHGobqDqX6tjLxJhv53ygjNtaaAn3sjeZ';

        res = await query(conn, alice, contract, "addressPassthrough", [default_account]);
        expect(res.output?.toJSON()).toEqual(default_account);

        res = await query(conn, alice, contract, "addressPassthrough", [dave.address]);
        expect(res.output?.toJSON()).toEqual(dave.address);

        res = await query(conn, alice, contract, "addressPassthrough", [alice.address]);
        expect(res.output?.toJSON()).toEqual(alice.address);
    });
});
