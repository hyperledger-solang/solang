import expect from 'expect';
import { establishConnection } from './index';
import crypto from 'crypto';

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

    it('store', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        let prog = await conn.loadProgram("store.so", "store.abi");

        // call the constructor
        await prog.call_constructor(conn, []);

        function returns_to_array(res: Object) {
            let arr = Object.values(res);
            let length = arr.pop()
            expect(arr.length).toEqual(length);
            return arr;
        }

        let res = returns_to_array(await prog.call_function(conn, "get_values1", []));

        expect(res).toStrictEqual(["0", "0", "0", "0"]);

        res = returns_to_array(await prog.call_function(conn, "get_values2", []));

        expect(res).toStrictEqual(["0", "", "0xb00b1e", "0x00000000", "0"]);

        await prog.call_function(conn, "set_values", []);

        res = returns_to_array(await prog.call_function(conn, "get_values1", []));

        expect(res).toStrictEqual([
            "18446744073709551615",
            "3671129839",
            "32766",
            "57896044618658097711785492504343953926634992332820282019728792003956564819967"
        ]);

        res = returns_to_array(await prog.call_function(conn, "get_values2", []));

        expect(res).toStrictEqual([
            "102",
            "the course of true love never did run smooth",
            "0xb00b1e",
            "0x41424344",
            "1",
        ]);

        await prog.call_function(conn, "do_ops", []);

        res = returns_to_array(await prog.call_function(conn, "get_values1", []));

        expect(res).toStrictEqual([
            "1",
            "65263",
            "32767",
            "57896044618658097711785492504343953926634992332820282019728792003956564819966",
        ]);

        res = returns_to_array(await prog.call_function(conn, "get_values2", []));

        expect(res).toStrictEqual([
            "61200",
            "",
            "0xb0ff1e",
            "0x61626364",
            "3",
        ]);

        await prog.call_function(conn, "push_zero", []);

        let bs = "0xb0ff1e00";

        for (let i = 0; i < 20; i++) {
            res = returns_to_array(await prog.call_function(conn, "get_bs", []));

            expect(res).toStrictEqual([bs]);

            if (bs.length <= 4 || Math.random() >= 0.5) {
                let val = ((Math.random() * 256) | 0).toString(16);

                val = val.length == 1 ? "0" + val : val;

                await prog.call_function(conn, "push", ["0x" + val]);

                bs += val;
            } else {
                res = returns_to_array(await prog.call_function(conn, "pop", []));

                let last = bs.slice(-2);

                expect(res).toStrictEqual(["0x" + last]);

                bs = bs.slice(0, -2);
            }

        }
    });

    it('structs', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        let prog = await conn.loadProgram("store.so", "store.abi");

        // call the constructor
        await prog.call_constructor(conn, []);

        function returns(res: Object) {
            let arr = Object.values(res);
            let length = arr.pop()
            expect(arr.length).toEqual(length);
            return JSON.stringify(arr);
        }

        await prog.call_function(conn, "set_foo1", []);

        // get foo1
        let res = returns(await prog.call_function(conn, "get_both_foos", []));

        // compare without JSON.stringify() results in "Received: serializes to the same string" error.
        // I have no idea why
        expect(res).toStrictEqual(JSON.stringify([
            [
                "1",
                "0x446f6e277420636f756e7420796f757220636869636b656e73206265666f72652074686579206861746368",
                "-102",
                "0xedaeda",
                "You can't have your cake and eat it too",
                [true, "There are other fish in the sea"]
            ],
            [
                "0",
                "0x",
                "0",
                "0x000000",
                "",
                [false, ""]
            ]
        ]));

        await prog.call_function(conn, "set_foo2", [
            [
                "1",
                "0xb52b073595ccb35eaebb87178227b779",
                "-123112321",
                "0x123456",
                "Barking up the wrong tree",
                [true, "Drive someone up the wall"]
            ],
            "nah"
        ]);

        res = returns(await prog.call_function(conn, "get_both_foos", []));

        expect(res).toStrictEqual(JSON.stringify([
            [
                "1",
                "0x446f6e277420636f756e7420796f757220636869636b656e73206265666f72652074686579206861746368",
                "-102",
                "0xedaeda",
                "You can't have your cake and eat it too",
                [true, "There are other fish in the sea"]
            ],
            [
                "1",
                "0xb52b073595ccb35eaebb87178227b779",
                "-123112321",
                "0x123456",
                "Barking up the wrong tree",
                [true, "nah"]
            ]
        ]));

        await prog.call_function(conn, "delete_foo", [true]);

        res = returns(await prog.call_function(conn, "get_foo", [false]));

        expect(res).toStrictEqual(JSON.stringify([
            [
                "1",
                "0xb52b073595ccb35eaebb87178227b779",
                "-123112321",
                "0x123456",
                "Barking up the wrong tree",
                [true, "nah"]
            ],
        ]));

        res = returns(await prog.call_function(conn, "get_foo", [true]));

        expect(res).toStrictEqual(JSON.stringify([
            [
                "0",
                "0x",
                "0",
                "0x000000",
                "",
                [false, ""]
            ],
        ]));

        await prog.call_function(conn, "delete_foo", [false]);

        res = returns(await prog.call_function(conn, "get_both_foos", []));

        // compare without JSON.stringify() results in "Received: serializes to the same string" error.
        // I have no idea why
        expect(res).toStrictEqual(JSON.stringify([
            [
                "0",
                "0x",
                "0",
                "0x000000",
                "",
                [false, ""]
            ],
            [
                "0",
                "0x",
                "0",
                "0x000000",
                "",
                [false, ""]
            ]
        ]));

        await prog.call_function(conn, "struct_literal", []);

        res = returns(await prog.call_function(conn, "get_foo", [true]));

        // compare without JSON.stringify() results in "Received: serializes to the same string" error.
        // I have no idea why
        expect(res).toStrictEqual(JSON.stringify([
            [
                "3",
                "0x537570657263616c6966726167696c697374696365787069616c69646f63696f7573",
                "64927",
                "0xe282ac",
                "Antidisestablishmentarianism",
                [true, "Pseudopseudohypoparathyroidism"],
            ]
        ]));
    });


    it('account storage too small constructor', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        // storage.sol needs 168 byes
        let prog = await conn.loadProgram("store.so", "store.abi", 512, 100);

        await expect(prog.call_constructor(conn, []))
            .rejects
            .toThrowError(new Error('failed to send transaction: Transaction simulation failed: Error processing Instruction 0: account data too small for instruction'));
    });

    it('returndata too small', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        // storage.sol needs 168 byes
        let prog = await conn.loadProgram("store.so", "store.abi", 512);

        await prog.call_constructor(conn, []);

        await prog.call_function(conn, "set_foo1", []);

        // get foo1
        await expect(prog.call_function(conn, "get_both_foos", []))
            .rejects
            .toThrowError(new Error('failed to send transaction: Transaction simulation failed: Error processing Instruction 0: account data too small for instruction'));
    });

    it('account storage too small dynamic alloc', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        // storage.sol needs 168 bytes on constructor, more for string data
        let prog = await conn.loadProgram("store.so", "store.abi", 512, 180);

        await prog.call_constructor(conn, []);

        // set a load of string which will overflow
        await expect(prog.call_function(conn, "set_foo1", []))
            .rejects
            .toThrowError(new Error('failed to send transaction: Transaction simulation failed: Error processing Instruction 0: account data too small for instruction'));
    });


    it('account storage too small dynamic realloc', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        // storage.sol needs 168 bytes on constructor, more for string data
        let prog = await conn.loadProgram("store.so", "store.abi", 512, 210);

        await prog.call_constructor(conn, []);

        async function push_until_bang() {
            for (let i = 0; i < 100; i++) {
                await prog.call_function(conn, "push", ["0x01"]);
                console.log("pushed one byte");
            }
        }

        // do realloc until failure
        await expect(push_until_bang())
            .rejects
            .toThrowError(new Error('failed to send transaction: Transaction simulation failed: Error processing Instruction 0: account data too small for instruction'));
    });

    it('arrays in account storage', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        // storage.sol needs 168 bytes on constructor, more for string data
        let prog = await conn.loadProgram("arrays.so", "arrays.abi", 512, 4096);

        await prog.call_constructor(conn, []);

        let users = [];

        for (let i = 0; i < 3; i++) {
            let addr = '0x' + crypto.randomBytes(32).toString('hex');
            let name = `name${i}`;
            let id = crypto.randomBytes(4).readUInt32BE(0).toString();
            let perms: string[] = [];

            for (let j = 0; j < Math.random() * 3; j++) {
                let p = Math.floor(Math.random() * 8);

                perms.push(`${p}`);
            }

            await prog.call_function(conn, "addUser", [id, addr, name, perms]);


            users.push([
                name, addr, id, perms
            ]);
        }

        function returns(res: Object) {
            let arr = Object.values(res);
            let length = arr.pop()
            expect(arr.length).toEqual(length);
            return JSON.stringify(arr);
        }

        let user = users[Math.floor(Math.random() * users.length)];

        let res = returns(await prog.call_function(conn, "getUserById", [user[2]]));

        expect(res).toStrictEqual(JSON.stringify([user]));

        if (user[3].length > 0) {
            let perms = user[3];

            let p = perms[Math.floor(Math.random() * perms.length)];

            res = returns(await prog.call_function(conn, "hasPermission", [user[2], p]));

            expect(res).toBe(JSON.stringify([true]));
        }

        user = users[Math.floor(Math.random() * users.length)];

        res = returns(await prog.call_function(conn, "getUserByAddress", [user[1]]));

        expect(res).toStrictEqual(JSON.stringify([user]));

        await prog.call_function(conn, "removeUser", [user[2]]);

        res = returns(await prog.call_function(conn, "userExists", [user[2]]));

        expect(res).toBe(JSON.stringify([false]));
    });

    it('external_call', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        let caller = await conn.loadProgram("caller.so", "caller.abi");
        let callee = await conn.loadProgram("callee.so", "callee.abi");

        // call the constructor
        await caller.call_constructor(conn, []);
        await callee.call_constructor(conn, []);

        await callee.call_function(conn, "set_x", ["102"]);

        let res = await callee.call_function(conn, "get_x", []);

        expect(res["0"]).toBe("102");

        let address = '0x' + callee.get_storage_key().toBuffer().toString('hex');
        console.log("addres: " + address);

        await caller.call_function(conn, "do_call", [address, "13123"], callee.all_keys());

        res = await callee.call_function(conn, "get_x", []);

        expect(res["0"]).toBe("13123");
    });
});
