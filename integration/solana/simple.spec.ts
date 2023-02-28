import expect from 'expect';
import { loadContract } from './setup';
import crypto from 'crypto';
import { BN } from '@project-serum/anchor';

describe('Simple solang tests', function () {
    this.timeout(500000);

    it('flipper', async function () {
        let { program, storage } = await loadContract('flipper', [true]);

        let res = await program.methods.get().accounts({ dataAccount: storage.publicKey }).view();

        expect(res).toStrictEqual(true);

        await program.methods.flip().accounts({ dataAccount: storage.publicKey }).rpc();
        res = await program.methods.get().accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toStrictEqual(false);
    });

    it('primitives', async function () {
        let { program, payer, storage } = await loadContract('primitives', []);

        // TEST Basic enums
        // in ethereum, an enum is described as an uint8 so can't use the enum
        // names programmatically. 0 = add, 1 = sub, 2 = mul, 3 = div, 4 = mod, 5 = pow, 6 = shl, 7 = shr
        let res = await program.methods.isMul({ mul: {} }).accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toBe(true);

        res = await program.methods.returnDiv().accounts({ dataAccount: storage.publicKey }).view();
        expect(res.div).toBeDefined();

        // TEST uint and int types, and arithmetic/bitwise ops
        res = await program.methods.opI64({ add: {} }, new BN(1000), new BN(4100)).view();
        expect(Number(res)).toBe(5100);
        res = await program.methods.opI64({ sub: {} }, new BN(1000), new BN(4100)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toStrictEqual(-3100);
        res = await program.methods.opI64({ mul: {} }, new BN(1000), new BN(4100)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(4100000);
        res = await program.methods.opI64({ div: {} }, new BN(1000), new BN(10)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(100);
        res = await program.methods.opI64({ mod: {} }, new BN(1000), new BN(99)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(10);
        res = await program.methods.opI64({ shl: {} }, new BN(-1000), new BN(8)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(-256000);
        res = await program.methods.opI64({ shr: {} }, new BN(-1000), new BN(8)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(-4);

        res = await program.methods.opU64({ add: {} }, new BN(1000), new BN(4100)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(5100);
        res = await program.methods.opU64({ sub: {} }, new BN(1000), new BN(4100)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(18446744073709548516); // (2^64)-18446744073709548516 = 3100
        res = await program.methods.opU64({ mul: {} }, new BN(123456789), new BN(123456789)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(15241578750190521);
        res = await program.methods.opU64({ div: {} }, new BN(123456789), new BN(100)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(1234567);
        res = await program.methods.opU64({ mod: {} }, new BN(123456789), new BN(100)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(89);
        res = await program.methods.opU64({ pow: {} }, new BN(3), new BN(7)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(2187);
        res = await program.methods.opI64({ shl: {} }, new BN(1000), new BN(8)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(256000);
        res = await program.methods.opI64({ shr: {} }, new BN(1000), new BN(8)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(3);

        // now for 256 bit operations
        res = await program.methods.opI256({ add: {} }, new BN(1000), new BN(4100)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(5100);
        res = await program.methods.opI256({ sub: {} }, new BN(1000), new BN(4100)).accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toStrictEqual(new BN(-3100));
        res = await program.methods.opI256({ mul: {} }, new BN(1000), new BN(4100)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(4100000);
        res = await program.methods.opI256({ div: {} }, new BN(1000), new BN(10)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(100);
        res = await program.methods.opI256({ mod: {} }, new BN(1000), new BN(99)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(10);
        res = await program.methods.opI256({ shl: {} }, new BN(-10000000000000), new BN(8)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(-2560000000000000);
        res = await program.methods.opI256({ shr: {} }, new BN(-10000000000000), new BN(8)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(-39062500000);

        res = await program.methods.opU256({ add: {} }, new BN(1000), new BN(4100)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(5100);
        res = await program.methods.opU256({ sub: {} }, new BN(1000), new BN(4100)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(115792089237316195423570985008687907853269984665640564039457584007913129636836); // (2^64)-18446744073709548516 = 3100
        res = await program.methods.opU256({ mul: {} }, new BN(123456789), new BN(123456789)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(15241578750190521);
        res = await program.methods.opU256({ div: {} }, new BN(123456789), new BN(100)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(1234567);
        res = await program.methods.opU256({ mod: {} }, new BN(123456789), new BN(100)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(89);
        res = await program.methods.opU256({ pow: {} }, new BN(123456789), new BN(9)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(6662462759719942007440037531362779472290810125440036903063319585255179509);
        res = await program.methods.opU256({ shl: {} }, new BN(10000000000000), new BN(8)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(2560000000000000);
        res = await program.methods.opU256({ shr: {} }, new BN(10000000000000), new BN(8)).accounts({ dataAccount: storage.publicKey }).view();
        expect(Number(res)).toBe(39062500000);

        // TEST bytesN
        res = await program.methods.returnU86().accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toStrictEqual([0x41, 0x42, 0x43, 0x44, 0x45, 0x46]);

        // TEST bytes5
        res = await program.methods.opU85Shift({ shl: {} },
            Buffer.from("deadcafe59", "hex"), new BN(8)).accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toStrictEqual([0xad, 0xca, 0xfe, 0x59, 0x00]);
        res = await program.methods.opU85Shift({ shr: {} }, Buffer.from("deadcafe59", "hex"), new BN(8)).accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toStrictEqual([0x00, 0xde, 0xad, 0xca, 0xfe]);
        res = await program.methods.opU85({ or: {} },
            Buffer.from("deadcafe59", "hex"),
            Buffer.from("0000000006", "hex")).accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toStrictEqual([0xde, 0xad, 0xca, 0xfe, 0x5f]);
        res = await program.methods.opU85({ and: {} },
            Buffer.from("deadcafe59", "hex"),
            Buffer.from("00000000ff", "hex")).accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toStrictEqual([0x00, 0x00, 0x00, 0x00, 0x59]);
        res = await program.methods.opU85({ xor: {} },
            Buffer.from("deadcafe59", "hex"),
            Buffer.from("00000000ff", "hex")).accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toStrictEqual([0xde, 0xad, 0xca, 0xfe, 0xa6]);

        // TEST bytes14
        res = await program.methods.opU814Shift({ shl: {} },
            Buffer.from("deadcafe123456789abcdefbeef7", "hex"), new BN(9))
            .accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toStrictEqual([0x5b, 0x95, 0xfc, 0x24, 0x68, 0xac, 0xf1, 0x35, 0x79, 0xbd, 0xf7, 0xdd, 0xee, 0x00]);
        res = await program.methods.opU814Shift({ shr: {} },
            Buffer.from("deadcafe123456789abcdefbeef7", "hex"), new BN(9)).accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toStrictEqual([0x00, 0x6f, 0x56, 0xe5, 0x7f, 0x09, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x7d, 0xf7]);
        res = await program.methods.opU814({ or: {} },
            Buffer.from("deadcafe123456789abcdefbeef7", "hex"),
            Buffer.from("0000060000000000000000000000", "hex")).accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toStrictEqual([0xde, 0xad, 0xce, 0xfe, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xfb, 0xee, 0xf7]);
        res = await program.methods.opU814({ and: {} },
            Buffer.from("deadcafe123456789abcdefbeef7", "hex"),
            Buffer.from("000000000000000000ff00000000", "hex")).accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toStrictEqual(
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xbc, 0x00, 0x00, 0x00, 0x00]);
        res = await program.methods.opU814({ xor: {} },
            Buffer.from("deadcafe123456789abcdefbeef7", "hex"),
            Buffer.from("ff00000000000000000000000000", "hex")).accounts({ dataAccount: storage.publicKey }).view();
        expect(res).toStrictEqual(
            [0x21, 0xad, 0xca, 0xfe, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xfb, 0xee, 0xf7]);

        res = await program.methods.addressPassthrough(payer.publicKey).view();
        expect(res).toStrictEqual(payer.publicKey);
    });

    it('store', async function () {
        const { storage, program } = await loadContract('store', []);

        let res = await program.methods.getValues1().accounts({ dataAccount: storage.publicKey }).view();

        expect(Number(res.return0)).toEqual(0);
        expect(Number(res.return1)).toEqual(0);
        expect(Number(res.return2)).toEqual(0);
        expect(Number(res.return3)).toEqual(0);

        res = await program.methods.getValues2().accounts({ dataAccount: storage.publicKey }).view();

        expect(Number(res.return0)).toEqual(0);
        expect(res.return1).toEqual("");
        expect(res.return2).toEqual(Buffer.from("b00b1e", "hex"));
        expect(res.return3).toEqual([0, 0, 0, 0]);
        expect(Number(res.return4.bar1)).toBeDefined();

        await program.methods.setValues().accounts({ dataAccount: storage.publicKey }).rpc();

        res = await program.methods.getValues1().accounts({ dataAccount: storage.publicKey }).view();

        expect(BigInt(res.return0)).toEqual(18446744073709551615n);
        expect(Number(res.return1)).toEqual(0xdad0feef);
        expect(Number(res.return2)).toEqual(0x7ffe);
        expect(BigInt(res.return3)).toEqual(57896044618658097711785492504343953926634992332820282019728792003956564819967n);

        res = await program.methods.getValues2().accounts({ dataAccount: storage.publicKey }).view();

        expect(Number(res.return0)).toEqual(102);
        expect(res.return1).toEqual("the course of true love never did run smooth");
        expect(res.return2).toEqual(Buffer.from("b00b1e", "hex"));
        expect(res.return3).toEqual([0x41, 0x42, 0x43, 0x44]);
        expect(Number(res.return4.bar2)).toBeDefined();

        await program.methods.doOps().accounts({ dataAccount: storage.publicKey }).rpc();

        res = await program.methods.getValues1().accounts({ dataAccount: storage.publicKey }).view();

        expect(BigInt(res.return0)).toEqual(1n);
        expect(Number(res.return1)).toEqual(65263);
        expect(Number(res.return2)).toEqual(32767);
        expect(BigInt(res.return3)).toEqual(57896044618658097711785492504343953926634992332820282019728792003956564819966n);

        res = await program.methods.getValues2().accounts({ dataAccount: storage.publicKey }).view();

        expect(Number(res.return0)).toEqual(61200);
        expect(res.return1).toEqual("");
        expect(res.return2).toEqual(Buffer.from("b0ff1e", "hex"));
        expect(res.return3).toEqual([0x61, 0x62, 0x63, 0x64]);
        expect(Number(res.return4.bar3)).toBeDefined();

        await program.methods.pushZero().accounts({ dataAccount: storage.publicKey }).rpc();

        let bs = "0xb0ff1e00";

        for (let i = 0; i < 20; i++) {
            res = await program.methods.getBs().accounts({ dataAccount: storage.publicKey }).view();

            expect(res).toStrictEqual(Buffer.from(bs.substring(2), "hex"));

            if (bs.length <= 4 || Math.random() >= 0.5) {
                let val = ((Math.random() * 256) | 0);

                await program.methods.push([val]).accounts({ dataAccount: storage.publicKey }).rpc();

                let valStr = val.toString(16);
                valStr = valStr.length == 1 ? "0" + valStr : valStr;

                bs += valStr;
            } else {
                await program.methods.pop().accounts({ dataAccount: storage.publicKey }).rpc();

                let last = bs.slice(-2);

                // TODO: rpc calls cannot return anything; this should be fixed in Anchor, so that
                // this line can be uncommented.
                //expect(res).toStrictEqual(Buffer.from(last, "hex"));

                bs = bs.slice(0, -2);
            }

        }
    });

    it('structs', async function () {
        const { program, storage } = await loadContract('store', []);

        await program.methods.setFoo1().accounts({ dataAccount: storage.publicKey }).rpc();

        // get foo1
        let res = await program.methods.getBothFoos().accounts({ dataAccount: storage.publicKey }).view();

        expect(res.return0.f1.bar2).toBeDefined();
        expect(res.return0.f2).toEqual(Buffer.from("Don't count your chickens before they hatch", "utf-8"));
        expect(res.return0.f3).toEqual(new BN(-102));
        expect(res.return0.f4).toEqual([0xed, 0xae, 0xda]);
        expect(res.return0.f5).toEqual("You can't have your cake and eat it too");
        expect(res.return0.f6.in1).toEqual(true);
        expect(res.return0.f6.in2).toEqual("There are other fish in the sea");

        expect(res.return1.f1.bar1).toBeDefined();
        expect(res.return1.f2).toEqual(Buffer.from([]));
        expect(res.return1.f3).toEqual(new BN(0));
        expect(res.return1.f4).toEqual([0, 0, 0]);
        expect(res.return1.f5).toEqual("");
        expect(res.return1.f6.in1).toEqual(false);
        expect(res.return1.f6.in2).toEqual("");

        await program.methods.setFoo2(
            {
                f1: { bar2: {} },
                f2: Buffer.from("b52b073595ccb35eaebb87178227b779", "hex"),
                f3: new BN(-123112321),
                f4: [0x12, 0x34, 0x56],
                f5: "Barking up the wrong tree",
                f6: { in1: true, in2: "Drive someone up the wall" }
            },
            "nah"
        ).accounts({ dataAccount: storage.publicKey }).rpc();

        res = await program.methods.getBothFoos().accounts({ dataAccount: storage.publicKey }).view();

        expect(res.return0.f1.bar2).toBeDefined();
        expect(res.return0.f2).toEqual(Buffer.from("Don't count your chickens before they hatch", "utf-8"));
        expect(res.return0.f3).toEqual(new BN(-102));
        expect(res.return0.f4).toEqual([0xed, 0xae, 0xda]);
        expect(res.return0.f5).toEqual("You can't have your cake and eat it too");
        expect(res.return0.f6.in1).toEqual(true);
        expect(res.return0.f6.in2).toEqual("There are other fish in the sea");

        expect(res.return1.f1.bar2).toBeDefined();
        expect(res.return1.f2).toEqual(Buffer.from("b52b073595ccb35eaebb87178227b779", "hex"));
        expect(res.return1.f3).toEqual(new BN(-123112321));
        expect(res.return1.f4).toEqual([0x12, 0x34, 0x56]);
        expect(res.return1.f5).toEqual("Barking up the wrong tree");
        expect(res.return1.f6.in1).toEqual(true);
        expect(res.return1.f6.in2).toEqual("nah");

        await program.methods.deleteFoo(true).accounts({ dataAccount: storage.publicKey }).rpc();

        res = await program.methods.getFoo(false).accounts({ dataAccount: storage.publicKey }).view();

        expect(res.f1.bar2).toBeDefined();
        expect(res.f2).toEqual(Buffer.from("b52b073595ccb35eaebb87178227b779", "hex"));
        expect(res.f3).toEqual(new BN(-123112321));
        expect(res.f4).toEqual([0x12, 0x34, 0x56]);
        expect(res.f5).toEqual("Barking up the wrong tree");
        expect(res.f6.in1).toEqual(true);
        expect(res.f6.in2).toEqual("nah");

        res = await program.methods.getFoo(true).accounts({ dataAccount: storage.publicKey }).view();

        expect(res.f1.bar1).toBeDefined();
        expect(res.f2).toEqual(Buffer.from([]));
        expect(res.f3).toEqual(new BN(0));
        expect(res.f4).toEqual([0, 0, 0]);
        expect(res.f5).toEqual("");
        expect(res.f6.in1).toEqual(false);
        expect(res.f6.in2).toEqual("");

        await program.methods.deleteFoo(false).accounts({ dataAccount: storage.publicKey }).rpc();

        res = await program.methods.getBothFoos().accounts({ dataAccount: storage.publicKey }).view();

        expect(res.return0.f1.bar1).toBeDefined();
        expect(res.return0.f2).toEqual(Buffer.from([]));
        expect(res.return0.f3).toEqual(new BN(0));
        expect(res.return0.f4).toEqual([0, 0, 0]);
        expect(res.return0.f5).toEqual("");
        expect(res.return0.f6.in1).toEqual(false);
        expect(res.return0.f6.in2).toEqual("");

        expect(res.return1.f1.bar1).toBeDefined();
        expect(res.return1.f2).toEqual(Buffer.from([]));
        expect(res.return1.f3).toEqual(new BN(0));
        expect(res.return1.f4).toEqual([0, 0, 0]);
        expect(res.return1.f5).toEqual("");
        expect(res.return1.f6.in1).toEqual(false);
        expect(res.return1.f6.in2).toEqual("");

        await program.methods.structLiteral().accounts({ dataAccount: storage.publicKey }).rpc();

        res = await program.methods.getFoo(true).accounts({ dataAccount: storage.publicKey }).view();

        expect(res.f1.bar4).toBeDefined();
        expect(res.f2).toEqual(Buffer.from("537570657263616c6966726167696c697374696365787069616c69646f63696f7573", "hex"));
        expect(res.f3).toEqual(new BN(64927));
        expect(res.f4).toEqual([0xe2, 0x82, 0xac]);
        expect(res.f5).toEqual("Antidisestablishmentarianism");
        expect(res.f6.in1).toEqual(true);
        expect(res.f6.in2).toEqual("Pseudopseudohypoparathyroidism");
    });


    it('account storage too small constructor', async function () {
        await expect(loadContract('store', [], 100))
            .rejects
            .toThrowError(new Error('failed to send transaction: Transaction simulation failed: Error processing Instruction 0: account data too small for instruction'));
    });

    it('account storage too small dynamic alloc', async function () {
        const { program, storage } = await loadContract('store', [], 233);

        // storage.sol needs 168 bytes on constructor, more for string data

        // set a load of string which will overflow
        await expect(program.methods.setFoo1().accounts({ dataAccount: storage.publicKey }).rpc())
            .rejects
            .toThrowError(new Error('failed to send transaction: Transaction simulation failed: Error processing Instruction 0: account data too small for instruction'));
    });

    it('account storage too small dynamic realloc', async function () {
        const { program, storage } = await loadContract('store', [], 233);

        async function push_until_bang() {
            for (let i = 0; i < 100; i++) {
                await program.methods.push(new Uint8Array([1])).accounts({ dataAccount: storage.publicKey }).rpc();
            }
        }

        // do realloc until failure
        await expect(push_until_bang())
            .rejects
            .toThrowError(new Error('failed to send transaction: Transaction simulation failed: Error processing Instruction 0: account data too small for instruction'));
    });

    it('arrays in account storage', async function () {
        const { program, storage } = await loadContract('arrays', []);

        let users = [];

        for (let i = 0; i < 3; i++) {
            let addr = [...crypto.randomBytes(32)];
            let name = `name${i}`;
            let id = new BN(crypto.randomBytes(4).readUInt32BE(0));
            let perms = [];

            for (let j = 0; j < Math.random() * 3; j++) {
                let p = Math.floor(Math.random() * 8);
                let perm = { [`perm${p + 1}`]: {} };

                perms.push(perm);
            }

            await program.methods.addUser(id, addr, name, perms)
                .accounts({ dataAccount: storage.publicKey })
                .rpc();

            users.push([
                name, addr, id, perms
            ]);
        }

        let user = users[Math.floor(Math.random() * users.length)];

        let res = await program.methods.getUserById(user[2]).accounts({ dataAccount: storage.publicKey }).view();

        expect(res.name).toEqual(user[0]);
        expect(res.addr).toEqual(user[1]);
        expect(res.id.cmp(user[2])).toBe(0);
        expect(res.perms).toEqual(user[3]);

        // @ts-ignore
        const perms: string[] = user[3];
        if (perms.length > 0) {

            let p = perms[Math.floor(Math.random() * perms.length)];

            res = await program.methods.hasPermission(user[2], p).accounts({ dataAccount: storage.publicKey }).view();

            expect(res).toStrictEqual(true);
        }

        user = users[Math.floor(Math.random() * users.length)];

        res = await program.methods.getUserByAddress(user[1]).accounts({ dataAccount: storage.publicKey }).view();

        expect(res.name).toEqual(user[0]);
        expect(res.addr).toEqual(user[1]);
        expect(res.id.cmp(user[2])).toBe(0);
        expect(res.perms).toEqual(user[3]);

        await program.methods.removeUser(user[2]).accounts({ dataAccount: storage.publicKey }).rpc();

        res = await program.methods.userExists(user[2]).accounts({ dataAccount: storage.publicKey }).view();

        expect(res).toStrictEqual(false);
    });
});
