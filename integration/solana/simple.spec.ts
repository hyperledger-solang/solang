import expect from 'expect';
import { loadContract } from './setup';
import crypto from 'crypto';

describe('Simple solang tests', function () {
    this.timeout(500000);

    it('flipper', async function () {
        let { contract } = await loadContract('flipper', [true]);

        let res = await contract.functions.get({ simulate: true });

        expect(res.result).toStrictEqual(true);

        await contract.functions.flip();

        res = await contract.functions.get({ simulate: true });

        expect(res.result).toStrictEqual(false);
    });

    it('primitives', async function () {
        let { contract, payer } = await loadContract('primitives', []);

        // TEST Basic enums
        // in ethereum, an enum is described as an uint8 so can't use the enum
        // names programmatically. 0 = add, 1 = sub, 2 = mul, 3 = div, 4 = mod, 5 = pow, 6 = shl, 7 = shr
        let res = await contract.functions.is_mul(2, { simulate: true });
        expect(res.result).toBe(true);

        res = await contract.functions.return_div({ simulate: true });
        expect(res.result).toBe(3);

        // TEST uint and int types, and arithmetic/bitwise ops
        res = await contract.functions.op_i64(0, 1000, 4100, { simulate: true });
        expect(Number(res.result)).toBe(5100);
        res = await contract.functions.op_i64(1, 1000, 4100, { simulate: true });
        expect(Number(res.result)).toStrictEqual(-3100);
        res = await contract.functions.op_i64(2, 1000, 4100, { simulate: true });
        expect(Number(res.result)).toBe(4100000);
        res = await contract.functions.op_i64(3, 1000, 10, { simulate: true });
        expect(Number(res.result)).toBe(100);
        res = await contract.functions.op_i64(4, 1000, 99, { simulate: true });
        expect(Number(res.result)).toBe(10);
        res = await contract.functions.op_i64(6, - 1000, 8, { simulate: true });
        expect(Number(res.result)).toBe(-256000);
        res = await contract.functions.op_i64(7, - 1000, 8, { simulate: true });
        expect(Number(res.result)).toBe(-4);


        res = await contract.functions.op_u64(0, 1000, 4100, { simulate: true });
        expect(Number(res.result)).toBe(5100);
        res = await contract.functions.op_u64(1, 1000, 4100, { simulate: true });
        expect(Number(res.result)).toBe(18446744073709548516); // (2^64)-18446744073709548516 = 3100
        res = await contract.functions.op_u64(2, 123456789, 123456789, { simulate: true });
        expect(Number(res.result)).toBe(15241578750190521);
        res = await contract.functions.op_u64(3, 123456789, 100, { simulate: true });
        expect(Number(res.result)).toBe(1234567);
        res = await contract.functions.op_u64(4, 123456789, 100, { simulate: true });
        expect(Number(res.result)).toBe(89);
        res = await contract.functions.op_u64(5, 3, 7, { simulate: true });
        expect(Number(res.result)).toBe(2187);
        res = await contract.functions.op_i64(6, 1000, 8, { simulate: true });
        expect(Number(res.result)).toBe(256000);
        res = await contract.functions.op_i64(7, 1000, 8, { simulate: true });
        expect(Number(res.result)).toBe(3);

        // now for 256 bit operations
        res = await contract.functions.op_i256(0, 1000, 4100, { simulate: true });
        expect(Number(res.result)).toBe(5100);
        res = await contract.functions.op_i256(1, 1000, 4100, { simulate: true });
        expect(res.result).toStrictEqual(BigInt(-3100));
        res = await contract.functions.op_i256(2, 1000, 4100, { simulate: true });
        expect(Number(res.result)).toBe(4100000);
        res = await contract.functions.op_i256(3, 1000, 10, { simulate: true });
        expect(Number(res.result)).toBe(100);
        res = await contract.functions.op_i256(4, 1000, 99, { simulate: true });
        expect(Number(res.result)).toBe(10);
        res = await contract.functions.op_i256(6, -10000000000000, 8, { simulate: true });
        expect(Number(res.result)).toBe(-2560000000000000);
        res = await contract.functions.op_i256(7, -10000000000000, 8, { simulate: true });
        expect(Number(res.result)).toBe(-39062500000);

        res = await contract.functions.op_u256(0, 1000, 4100, { simulate: true });
        expect(Number(res.result)).toBe(5100);
        res = await contract.functions.op_u256(1, 1000, 4100, { simulate: true });
        expect(Number(res.result)).toBe(115792089237316195423570985008687907853269984665640564039457584007913129636836); // (2^64)-18446744073709548516 = 3100
        res = await contract.functions.op_u256(2, 123456789, 123456789, { simulate: true });
        expect(Number(res.result)).toBe(15241578750190521);
        res = await contract.functions.op_u256(3, 123456789, 100, { simulate: true });
        expect(Number(res.result)).toBe(1234567);
        res = await contract.functions.op_u256(4, 123456789, 100, { simulate: true });
        expect(Number(res.result)).toBe(89);
        res = await contract.functions.op_u256(5, 123456789, 9, { simulate: true });
        expect(Number(res.result)).toBe(6662462759719942007440037531362779472290810125440036903063319585255179509);
        res = await contract.functions.op_u256(6, 10000000000000, 8, { simulate: true });
        expect(Number(res.result)).toBe(2560000000000000);
        res = await contract.functions.op_u256(7, 10000000000000, 8, { simulate: true });
        expect(Number(res.result)).toBe(39062500000);


        // TEST bytesN
        res = await contract.functions.return_u8_6({ simulate: true });
        expect(res.result).toStrictEqual(new Uint8Array(Buffer.from("414243444546", "hex")));

        // TEST bytes5
        res = await contract.functions.op_u8_5_shift(6,
            new Uint8Array(Buffer.from("deadcafe59", "hex")), 8, { simulate: true });
        expect(res.result).toStrictEqual(new Uint8Array(Buffer.from("adcafe5900", "hex")));
        res = await contract.functions.op_u8_5_shift(7, new Uint8Array(Buffer.from("deadcafe59", "hex")), 8, { simulate: true });
        expect(res.result).toStrictEqual(new Uint8Array(Buffer.from("00deadcafe", "hex")));
        res = await contract.functions.op_u8_5(8,
            new Uint8Array(Buffer.from("deadcafe59", "hex")),
            new Uint8Array(Buffer.from("0000000006", "hex")), { simulate: true });
        expect(res.result).toStrictEqual(new Uint8Array(Buffer.from("deadcafe5f", "hex")));
        res = await contract.functions.op_u8_5(9,
            new Uint8Array(Buffer.from("deadcafe59", "hex")),
            new Uint8Array(Buffer.from("00000000ff", "hex")), { simulate: true });
        expect(res.result).toStrictEqual(
            new Uint8Array(Buffer.from("0000000059", "hex")));
        res = await contract.functions.op_u8_5(10,
            new Uint8Array(Buffer.from("deadcafe59", "hex")),
            new Uint8Array(Buffer.from("00000000ff", "hex")), { simulate: true });
        expect(res.result).toStrictEqual(new Uint8Array(Buffer.from("deadcafea6", "hex")));

        // TEST bytes14
        res = await contract.functions.op_u8_14_shift(6,
            new Uint8Array(Buffer.from("deadcafe123456789abcdefbeef7", "hex")), 9, { simulate: true });
        expect(res.result).toStrictEqual(new Uint8Array(Buffer.from("5b95fc2468acf13579bdf7ddee00", "hex")));
        res = await contract.functions.op_u8_14_shift(7,
            new Uint8Array(Buffer.from("deadcafe123456789abcdefbeef7", "hex")), 9, { simulate: true });
        expect(res.result).toStrictEqual(new Uint8Array(Buffer.from("006f56e57f091a2b3c4d5e6f7df7", "hex")));
        res = await contract.functions.op_u8_14(8,
            new Uint8Array(Buffer.from("deadcafe123456789abcdefbeef7", "hex")),
            new Uint8Array(Buffer.from("0000060000000000000000000000", "hex")), { simulate: true });
        expect(res.result).toStrictEqual(new Uint8Array(Buffer.from("deadcefe123456789abcdefbeef7", "hex")));
        res = await contract.functions.op_u8_14(9,
            new Uint8Array(Buffer.from("deadcafe123456789abcdefbeef7", "hex")),
            new Uint8Array(Buffer.from("000000000000000000ff00000000", "hex")), { simulate: true });
        expect(res.result).toStrictEqual(
            new Uint8Array(Buffer.from("000000000000000000bc00000000", "hex")));
        res = await contract.functions.op_u8_14(10,
            new Uint8Array(Buffer.from("deadcafe123456789abcdefbeef7", "hex")),
            new Uint8Array(Buffer.from("ff00000000000000000000000000", "hex")), { simulate: true });
        expect(res.result).toStrictEqual(
            new Uint8Array(Buffer.from("21adcafe123456789abcdefbeef7", "hex")));

        res = await contract.functions.address_passthrough(payer.publicKey.toBytes());
        expect(res.result).toStrictEqual(new Uint8Array(payer.publicKey.toBytes()));
    });

    it('store', async function () {
        const { contract } = await loadContract('store', []);

        let res = await contract.functions.get_values1({ simulate: true });

        expect(res.result.toString()).toEqual("0,0,0,0");

        res = await contract.functions.get_values2({ simulate: true });

        expect(res.result.toString()).toStrictEqual(
            [
                0,
                "",
                new Uint8Array(Buffer.from("b00b1e", "hex")),
                new Uint8Array([0, 0, 0, 0]),
                0
            ].toString()
        );

        await contract.functions.set_values();

        res = await contract.functions.get_values1({ simulate: true });

        expect(res.result.toString()).toStrictEqual('18446744073709551615,3671129839,32766,57896044618658097711785492504343953926634992332820282019728792003956564819967');

        res = await contract.functions.get_values2({ simulate: true });

        expect(res.result.toString()).toStrictEqual(
            [
                102,
                "the course of true love never did run smooth",
                new Uint8Array(Buffer.from("b00b1e", "hex")),
                new Uint8Array(Buffer.from("41424344", "hex")),
                1
            ].toString()
        );

        await contract.functions.do_ops();

        res = await contract.functions.get_values1({ simulate: true });

        expect(res.result.toString()).toStrictEqual("1,65263,32767,57896044618658097711785492504343953926634992332820282019728792003956564819966");

        res = await contract.functions.get_values2({ simulate: true });

        expect(res.result.toString()).toStrictEqual(
            [
                61200,
                "",
                new Uint8Array(Buffer.from("b0ff1e", "hex")),
                new Uint8Array(Buffer.from("61626364", "hex")),
                3
            ].toString()
        );

        await contract.functions.push_zero();

        let bs = "0xb0ff1e00";

        for (let i = 0; i < 20; i++) {
            res = await contract.functions.get_bs({ simulate: true });

            expect(res.result).toStrictEqual(new Uint8Array(Buffer.from(bs.substring(2), "hex")));

            if (bs.length <= 4 || Math.random() >= 0.5) {
                let val = ((Math.random() * 256) | 0);

                await contract.functions.push(new Uint8Array([val]));

                let valStr = val.toString(16);
                valStr = valStr.length == 1 ? "0" + valStr : valStr;

                bs += valStr;
            } else {
                res = await contract.functions.pop();

                let last = bs.slice(-2);

                expect(res.result).toStrictEqual(
                    new Uint8Array(Buffer.from(last, "hex")));

                bs = bs.slice(0, -2);
            }

        }
    });

    it('structs', async function () {
        const { contract } = await loadContract('store', []);

        await contract.functions.set_foo1();

        // get foo1
        let res = await contract.functions.get_both_foos({ simulate: true });


        expect(res.result[0].toString()).toStrictEqual(
            [
                1,
                new Uint8Array(Buffer.from("Don't count your chickens before they hatch", "utf-8")),
                -102,
                new Uint8Array(Buffer.from("edaeda", "hex")),
                "You can't have your cake and eat it too",
                [true, "There are other fish in the sea"]
            ].toString());

        expect(res.result[1].toString()).toStrictEqual(
            [
                0,
                new Uint8Array([]),
                0,
                new Uint8Array([0, 0, 0]),
                "",
                [false, ""]
            ].toString());

        await contract.functions.set_foo2(
            [
                1,
                new Uint8Array(Buffer.from("b52b073595ccb35eaebb87178227b779", "hex")),
                Number("-123112321"),
                new Uint8Array(Buffer.from("123456", "hex")),
                "Barking up the wrong tree",
                [true, "Drive someone up the wall"]
            ],
            "nah"
        );

        res = await contract.functions.get_both_foos({ simulate: true });
        expect(res.result[0].toString()).toStrictEqual(
            [
                1,
                new Uint8Array(Buffer.from("Don't count your chickens before they hatch", "utf-8")),
                -102,
                new Uint8Array(Buffer.from("edaeda", "hex")),
                "You can't have your cake and eat it too",
                [true, "There are other fish in the sea"]
            ].toString());
        expect(res.result[1].toString()).toStrictEqual(
            [
                1,
                new Uint8Array(Buffer.from("b52b073595ccb35eaebb87178227b779", "hex")),
                -123112321,
                new Uint8Array(Buffer.from("123456", "hex")),
                "Barking up the wrong tree",
                [true, "nah"]
            ].toString());

        await contract.functions.delete_foo(true);

        res = await contract.functions.get_foo(false, { simulate: true });

        expect(res.result.toString()).toStrictEqual(
            [
                1,
                new Uint8Array(Buffer.from("b52b073595ccb35eaebb87178227b779", "hex")),
                -123112321,
                new Uint8Array(Buffer.from("123456", "hex")),
                "Barking up the wrong tree",
                [true, "nah"]
            ].toString());

        res = await contract.functions.get_foo(true, { simulate: true });

        expect(res.result.toString()).toStrictEqual(([
            [
                0,
                new Uint8Array([]),
                0,
                new Uint8Array([0, 0, 0]),
                "",
                [false, ""]
            ],
        ]).toString());

        await contract.functions.delete_foo(false);

        res = await contract.functions.get_both_foos({ simulate: true });

        expect(res.result[0].toString()).toStrictEqual(([
            [
                0,
                new Uint8Array([]),
                0,
                new Uint8Array([0, 0, 0]),
                "",
                [false, ""]
            ],
        ]).toString());

        expect(res.result[1].toString()).toStrictEqual(([
            [
                0,
                new Uint8Array([]),
                0,
                new Uint8Array([0, 0, 0]),
                "",
                [false, ""]
            ],
        ]).toString());

        await contract.functions.struct_literal();

        res = await contract.functions.get_foo(true, { simulate: true });

        // compare without JSON.stringify() results in "Received: serializes to the same string" error.
        // I have no idea why
        expect(res.result.toString()).toStrictEqual(
            [
                3,
                new Uint8Array(Buffer.from("537570657263616c6966726167696c697374696365787069616c69646f63696f7573", "hex")),
                64927,
                new Uint8Array(Buffer.from("e282ac", "hex")),
                "Antidisestablishmentarianism",
                [true, "Pseudopseudohypoparathyroidism"],
            ]
                .toString());
    });


    it('account storage too small constructor', async function () {
        await expect(loadContract('store', [], 100))
            .rejects
            .toThrowError(new Error('account data too small for instruction'));
    });

    it('account storage too small dynamic alloc', async function () {
        const { contract } = await loadContract('store', [], 233);

        // storage.sol needs 168 bytes on constructor, more for string data

        // set a load of string which will overflow
        await expect(contract.functions.set_foo1())
            .rejects
            .toThrowError(new Error('account data too small for instruction'));
    });

    it('account storage too small dynamic realloc', async function () {
        const { contract } = await loadContract('store', [], 233);

        async function push_until_bang() {
            for (let i = 0; i < 100; i++) {
                await contract.functions.push(new Uint8Array([1]));
            }
        }

        // do realloc until failure
        await expect(push_until_bang())
            .rejects
            .toThrowError(new Error('account data too small for instruction'));
    });

    it('arrays in account storage', async function () {
        const { contract } = await loadContract('arrays', []);

        let users = [];

        for (let i = 0; i < 3; i++) {
            let addr = new Uint8Array(crypto.randomBytes(32));
            let name = `name${i}`;
            let id = crypto.randomBytes(4).readUInt32BE(0);
            let perms: string[] = [];

            for (let j = 0; j < Math.random() * 3; j++) {
                let p = Math.floor(Math.random() * 8);

                perms.push(`${p}`);
            }

            await contract.functions.addUser(id, addr, name, perms);


            users.push([
                name, addr, id, perms
            ]);
        }

        let user = users[Math.floor(Math.random() * users.length)];

        let res = await contract.functions.getUserById(user[2], { simulate: true });

        expect(res.result.toString()).toStrictEqual(user.toString());

        // @ts-ignore
        const perms: string[] = user[3];
        if (perms.length > 0) {

            let p = perms[Math.floor(Math.random() * perms.length)];

            res = await contract.functions.hasPermission(user[2], p, { simulate: true });

            expect(res.result).toStrictEqual(true);
        }

        user = users[Math.floor(Math.random() * users.length)];

        res = await contract.functions.getUserByAddress(user[1], { simulate: true });


        expect(res.result.toString()).toStrictEqual(
            user.toString()
        );

        await contract.functions.removeUser(user[2]);

        res = await contract.functions.userExists(user[2]);

        expect(res.result).toStrictEqual(false);
    });
});
