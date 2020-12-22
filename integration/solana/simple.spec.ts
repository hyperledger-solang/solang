import expect from 'expect';
import { establishConnection } from './index';

describe('Deploy solang contract and test', () => {
    it('flipper', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        let prog = await conn.loadProgram("flipper.so", "flipper.abi");

        // call the constructor
        await prog.call_constructor(conn, ["true"]);

        let res = await prog.call_function(conn, "get", []);

        expect(res["__length__"]).toBe(1);
        expect(res["0"]).toBe(true);

        await prog.call_function(conn, "flip", []);

        res = await prog.call_function(conn, "get", []);

        expect(res["__length__"]).toBe(1);
        expect(res["0"]).toBe(false);
    });

    it('primitives', async function () {
        this.timeout(100000);

        let conn = await establishConnection();

        let prog = await conn.loadProgram("primitives.so", "primitives.abi");

        // call the constructor
        await prog.call_constructor(conn, []);

        // TEST Basic enums
        // in ethereum, an enum is described as an uint8 so can't use the enum
        // names programmatically. 0 = add, 1 = sub, 2 = mul, 3 = div, 4 = mod, 5 = pow, 6 = shl, 7 = shr
        let res = await prog.call_function(conn, "is_mul", ["2"]);
        expect(res["0"]).toBe(true);

        res = await prog.call_function(conn, "return_div", []);
        expect(res["0"]).toBe("3");

        // TEST uint and int types, and arithmetic/bitwise ops
        res = await prog.call_function(conn, "op_i64", ["0", "1000", "4100"]);
        expect(res["0"]).toBe("5100");
        res = await prog.call_function(conn, "op_i64", ["1", "1000", "4100"]);
        expect(res["0"]).toBe("-3100");
        res = await prog.call_function(conn, "op_i64", ["2", "1000", "4100"]);
        expect(res["0"]).toBe("4100000");
        res = await prog.call_function(conn, "op_i64", ["3", "1000", "10"]);
        expect(res["0"]).toBe("100");
        res = await prog.call_function(conn, "op_i64", ["4", "1000", "99"]);
        expect(res["0"]).toBe("10");
        res = await prog.call_function(conn, "op_i64", ["6", "-1000", "8"]);
        expect(res["0"]).toBe("-256000");
        res = await prog.call_function(conn, "op_i64", ["7", "-1000", "8"]);
        expect(res["0"]).toBe("-4");


        res = await prog.call_function(conn, "op_u64", ["0", "1000", "4100"]);
        expect(res["0"]).toBe("5100");
        res = await prog.call_function(conn, "op_u64", ["1", "1000", "4100"]);
        expect(res["0"]).toBe("18446744073709548516"); // (2^64)-18446744073709548516 = 3100
        res = await prog.call_function(conn, "op_u64", ["2", "123456789", "123456789"]);
        expect(res["0"]).toBe("15241578750190521");
        res = await prog.call_function(conn, "op_u64", ["3", "123456789", "100"]);
        expect(res["0"]).toBe("1234567");
        res = await prog.call_function(conn, "op_u64", ["4", "123456789", "100"]);
        expect(res["0"]).toBe("89");
        res = await prog.call_function(conn, "op_u64", ["5", "3", "7"]);
        expect(res["0"]).toBe("2187");
        res = await prog.call_function(conn, "op_i64", ["6", "1000", "8"]);
        expect(res["0"]).toBe("256000");
        res = await prog.call_function(conn, "op_i64", ["7", "1000", "8"]);
        expect(res["0"]).toBe("3");

        // now for 256 bit operations
        res = await prog.call_function(conn, "op_i256", ["0", "1000", "4100"]);
        expect(res["0"]).toBe("5100");
        res = await prog.call_function(conn, "op_i256", ["1", "1000", "4100"]);
        expect(res["0"]).toBe("-3100");
        res = await prog.call_function(conn, "op_i256", ["2", "1000", "4100"]);
        expect(res["0"]).toBe("4100000");
        res = await prog.call_function(conn, "op_i256", ["3", "1000", "10"]);
        expect(res["0"]).toBe("100");
        res = await prog.call_function(conn, "op_i256", ["4", "1000", "99"]);
        expect(res["0"]).toBe("10");
        res = await prog.call_function(conn, "op_i256", ["6", "-10000000000000", "8"]);
        expect(res["0"]).toBe("-2560000000000000");
        res = await prog.call_function(conn, "op_i256", ["7", "-10000000000000", "8"]);
        expect(res["0"]).toBe("-39062500000");

        res = await prog.call_function(conn, "op_u256", ["0", "1000", "4100"]);
        expect(res["0"]).toBe("5100");
        res = await prog.call_function(conn, "op_u256", ["1", "1000", "4100"]);
        expect(res["0"]).toBe("115792089237316195423570985008687907853269984665640564039457584007913129636836"); // (2^64)-18446744073709548516 = 3100
        res = await prog.call_function(conn, "op_u256", ["2", "123456789", "123456789"]);
        expect(res["0"]).toBe("15241578750190521");
        res = await prog.call_function(conn, "op_u256", ["3", "123456789", "100"]);
        expect(res["0"]).toBe("1234567");
        res = await prog.call_function(conn, "op_u256", ["4", "123456789", "100"]);
        expect(res["0"]).toBe("89");
        res = await prog.call_function(conn, "op_u256", ["5", "123456789", "9"]);
        expect(res["0"]).toBe("6662462759719942007440037531362779472290810125440036903063319585255179509");
        res = await prog.call_function(conn, "op_i256", ["6", "10000000000000", "8"]);
        expect(res["0"]).toBe("2560000000000000");
        res = await prog.call_function(conn, "op_i256", ["7", "10000000000000", "8"]);
        expect(res["0"]).toBe("39062500000");


        // TEST bytesN
        res = await prog.call_function(conn, "return_u8_6", []);
        expect(res["0"]).toBe("0x414243444546");

        // TEST bytes5
        res = await prog.call_function(conn, "op_u8_5_shift", ["6", "0xdeadcafe59", "8"]);
        expect(res["0"]).toBe("0xadcafe5900");
        res = await prog.call_function(conn, "op_u8_5_shift", ["7", "0xdeadcafe59", "8"]);
        expect(res["0"]).toBe("0x00deadcafe");
        res = await prog.call_function(conn, "op_u8_5", ["8", "0xdeadcafe59", "0x0000000006"]);
        expect(res["0"]).toBe("0xdeadcafe5f");
        res = await prog.call_function(conn, "op_u8_5", ["9", "0xdeadcafe59", "0x00000000ff"]);
        expect(res["0"]).toBe("0x0000000059");
        res = await prog.call_function(conn, "op_u8_5", ["10", "0xdeadcafe59", "0x00000000ff"]);
        expect(res["0"]).toBe("0xdeadcafea6");

        // TEST bytes14
        res = await prog.call_function(conn, "op_u8_14_shift", ["6", "0xdeadcafe123456789abcdefbeef7", "9"]);
        expect(res["0"]).toBe("0x5b95fc2468acf13579bdf7ddee00");
        res = await prog.call_function(conn, "op_u8_14_shift", ["7", "0xdeadcafe123456789abcdefbeef7", "9"]);
        expect(res["0"]).toBe("0x006f56e57f091a2b3c4d5e6f7df7");
        res = await prog.call_function(conn, "op_u8_14", ["8", "0xdeadcafe123456789abcdefbeef7", "0x00000600"]);
        expect(res["0"]).toBe("0xdeadcefe123456789abcdefbeef7");
        res = await prog.call_function(conn, "op_u8_14", ["9", "0xdeadcafe123456789abcdefbeef7", "0x000000000000000000ff"]);
        expect(res["0"]).toBe("0x000000000000000000bc00000000");
        res = await prog.call_function(conn, "op_u8_14", ["10", "0xdeadcafe123456789abcdefbeef7", "0xff"]);
        expect(res["0"]).toBe("0x21adcafe123456789abcdefbeef7");

        // TEST address type. We need to encoding this has a hex string with the '0x' prefix, since solang maps address
        // to bytes32 type
        let address = '0x' + conn.payerAccount.publicKey.toBuffer().toString('hex');
        console.log(`Using address ${address} for testing`)
        res = await prog.call_function(conn, "address_passthrough", [address]);
        expect(res["0"]).toBe(address);
    });
});
