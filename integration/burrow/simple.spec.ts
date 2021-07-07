import { strictEqual } from 'assert';
import { Contract, Burrow } from '@hyperledger/burrow';
import { readFileSync } from 'fs';
import { BigNumber } from 'ethers/lib/ethers';

const default_url: string = "localhost:10997";
const default_account = 'ABE2314B5D38BE9EA2BEDB8E58345C62FA6636BA';

export async function establishConnection(): Promise<Burrow> {
    let url = process.env.RPC_URL || default_url;
    return new Burrow(url, default_account);
}

describe('Deploy solang contract and test', () => {
    it('flipper', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        const bytecode: string = readFileSync('flipper.wasm').toString('hex');

        const abi = JSON.parse(readFileSync('flipper.abi', 'utf-8'));

        let contract = new Contract({ abi, bytecode });

        let prog: any = await contract.deploy(conn, false);

        let output = await prog.get();
        strictEqual(output[0], false);

        await prog.flip();

        output = await prog.get();
        strictEqual(output[0], true);
    });

    it('flipper-true', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        const bytecode: string = readFileSync('flipper.wasm').toString('hex');

        const abi = JSON.parse(readFileSync('flipper.abi', 'utf-8'));

        let contract = new Contract({ abi, bytecode });

        let prog: any = await contract.deploy(conn, true);

        let output = await prog.get();
        strictEqual(output[0], true);

        await prog.flip();

        output = await prog.get();
        strictEqual(output[0], false);
    });

    it('primitives', async function () {
        this.timeout(100000);

        let conn = await establishConnection();

        const bytecode: string = readFileSync('primitives.wasm').toString('hex');

        const abi = JSON.parse(readFileSync('primitives.abi', 'utf-8'));

        // call the constructor
        let contract = new Contract({ abi, bytecode });

        let prog: any = await contract.deploy(conn);

        // TEST Basic enums
        // in ethereum, an enum is described as an uint8 so can't use the enum
        // names programmatically. 0 = add, 1 = sub, 2 = mul, 3 = div, 4 = mod, 5 = pow, 6 = shl, 7 = shr
        let res = await prog.is_mul(2);
        strictEqual(res[0], true);

        res = await prog.return_div();
        strictEqual(res[0], 3);

        // TEST uint and int types, and arithmetic/bitwise ops
        res = await prog.op_i64(0, 1000, 4100);
        strictEqual(res[0], 5100);
        res = await prog.op_i64(1, 1000, 4100);
        strictEqual(res[0], -3100);
        res = await prog.op_i64(2, 1000, 4100);
        strictEqual(res[0], 4100000);
        res = await prog.op_i64(3, 1000, 10);
        strictEqual(res[0], 100);
        res = await prog.op_i64(4, 1000, 99);
        strictEqual(res[0], 10);
        res = await prog.op_i64(6, - 1000, 8);
        strictEqual(res[0], -256000);
        res = await prog.op_i64(7, - 1000, 8);
        strictEqual(res[0], -4);


        res = await prog.op_u64(0, 1000, 4100);
        strictEqual(res[0], 5100);
        res = await prog.op_u64(1, 1000, 4100);
        strictEqual(BigNumber.from('18446744073709548516').eq(res[0]), true); // (2^64)-18446744073709548516 = 3100
        res = await prog.op_u64(2, 123456789, 123456789);
        strictEqual(BigNumber.from('15241578750190521').eq(res[0]), true);
        res = await prog.op_u64(3, 123456789, 100);
        strictEqual(res[0], 1234567);
        res = await prog.op_u64(4, 123456789, 100);
        strictEqual(res[0], 89);
        res = await prog.op_u64(5, 3, 7);
        strictEqual(res[0], 2187);
        res = await prog.op_i64(6, 1000, 8);
        strictEqual(res[0], 256000);
        res = await prog.op_i64(7, 1000, 8);
        strictEqual(res[0], 3);

        // now for 256 bit operations
        res = await prog.op_i256(0, 1000, 4100);
        strictEqual(res[0], 5100);
        res = await prog.op_i256(1, 1000, 4100);
        strictEqual(res[0], -3100);
        res = await prog.op_i256(2, 1000, 4100);
        strictEqual(res[0], 4100000);
        res = await prog.op_i256(3, 1000, 10);
        strictEqual(res[0], 100);
        res = await prog.op_i256(4, 1000, 99);
        strictEqual(res[0], 10);
        res = await prog.op_i256(6, - 10000000000000, 8);
        strictEqual(res[0], -2560000000000000);
        res = await prog.op_i256(7, - 10000000000000, 8);
        strictEqual(res[0], -39062500000);

        res = await prog.op_u256(0, 1000, 4100);
        strictEqual(res[0], 5100);
        res = await prog.op_u256(1, 1000, 4100);
        strictEqual(BigNumber.from('115792089237316195423570985008687907853269984665640564039457584007913129636836').eq(res[0]), true); // (2^64)-18446744073709548516 = 3100
        res = await prog.op_u256(2, 123456789, 123456789);
        strictEqual(BigNumber.from('15241578750190521').eq(res[0]), true);
        res = await prog.op_u256(3, 123456789, 100);
        strictEqual(res[0], 1234567);
        res = await prog.op_u256(4, 123456789, 100);
        strictEqual(res[0], 89);
        res = await prog.op_u256(5, 123456789, 9);
        strictEqual(BigNumber.from('6662462759719942007440037531362779472290810125440036903063319585255179509').eq(res[0]), true);
        res = await prog.op_i256(6, 10000000000000, 8);
        strictEqual(res[0].toString(), '2560000000000000');
        res = await prog.op_i256(7, 10000000000000, 8);
        strictEqual(res[0].toString(), '39062500000');


        // TEST bytesN
        res = await prog.return_u8_6();
        strictEqual(res[0], '414243444546');

        // // TEST bytes5
        res = await prog.op_u8_5_shift(6, 'deadcafe59', 8);
        strictEqual(res[0], 'ADCAFE5900');
        res = await prog.op_u8_5_shift(7, 'deadcafe59', 8);
        strictEqual(res[0], '00DEADCAFE');
        res = await prog.op_u8_5(8, 'deadcafe59', '0000000006');
        strictEqual(res[0], 'DEADCAFE5F');
        res = await prog.op_u8_5(9, 'deadcafe59', '00000000ff');
        strictEqual(res[0], '0000000059');
        res = await prog.op_u8_5(10, 'deadcafe59', '00000000ff');
        strictEqual(res[0], 'DEADCAFEA6');

        // TEST bytes14
        res = await prog.op_u8_14_shift(6, 'deadcafe123456789abcdefbeef7', 9);
        strictEqual(res[0], '5B95FC2468ACF13579BDF7DDEE00');
        res = await prog.op_u8_14_shift(7, 'deadcafe123456789abcdefbeef7', 9);
        strictEqual(res[0], '006F56E57F091A2B3C4D5E6F7DF7');
        res = await prog.op_u8_14(8, 'deadcafe123456789abcdefbeef7', '0000060000000000000000000000');
        strictEqual(res[0], 'DEADCEFE123456789ABCDEFBEEF7');
        res = await prog.op_u8_14(9, 'deadcafe123456789abcdefbeef7', '000000000000000000ff00000000');
        strictEqual(res[0], '000000000000000000BC00000000');
        res = await prog.op_u8_14(10, 'deadcafe123456789abcdefbeef7', 'ff00000000000000000000000000');
        strictEqual(res[0], '21ADCAFE123456789ABCDEFBEEF7');

        // TEST address type.
        res = await prog.address_passthrough(default_account);
        strictEqual(res[0], default_account);
    });
});
