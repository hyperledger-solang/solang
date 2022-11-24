import expect from 'expect';
import { weight, createConnection, deploy, transaction, aliceKeypair, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';

// TODO:
// This apparently works with subxt.
describe.skip('Deploy store contract and test', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('store', async function () {
        this.timeout(100000);

        const alice = aliceKeypair();

        let deployed_contract = await deploy(conn, alice, 'store.contract', BigInt(0));

        let contract = new ContractPromise(conn, deployed_contract.abi, deployed_contract.address);

        let res1 = await contract.query.getValues1(alice.address, {});

        expect(res1.output?.toJSON()).toStrictEqual([0, 0, 0, 0]);

        let res2 = await contract.query.getValues2(alice.address, {});

        expect(res2.output?.toJSON()).toStrictEqual([0, "", "0xb00b1e", "0x00000000", "bar1"]);

        var gasLimit = await weight(conn, contract, "setValues");
        const tx1 = contract.tx.setValues({ gasLimit });

        await transaction(tx1, alice);

        let res3 = await contract.query.getValues1(alice.address, {});

        expect(res3.output?.toJSON()).toStrictEqual(["0xffffffffffffffff",
            3671129839,
            32766,
            "0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        ]);

        let res4 = await contract.query.getValues2(alice.address, {});

        expect(res4.output?.toJSON()).toStrictEqual([
            102,
            "the course of true love never did run smooth",
            "0xb00b1e",
            "0x41424344",
            "bar2",
        ]);

        var gasLimit = await weight(conn, contract, "doOps");
        const tx2 = contract.tx.doOps({ gasLimit });

        await transaction(tx2, alice);

        let res5 = await contract.query.getValues1(alice.address, {});

        expect(res5.output?.toJSON()).toStrictEqual([
            1,
            65263,
            32767,
            "0x7ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe",
        ]);

        let res6 = await contract.query.getValues2(alice.address, {});

        expect(res6.output?.toJSON()).toStrictEqual([
            61200,
            "",
            "0xb0ff1e",
            "0x61626364",
            "bar4",
        ]);

        var gasLimit = await weight(conn, contract, "pushZero");
        const tx3 = contract.tx.pushZero({ gasLimit });

        await transaction(tx3, alice);

        let bs = "0xb0ff1e00";

        for (let i = 0; i < 20; i++) {
            let res7 = await contract.query.getBs(alice.address, {});

            expect(res7.output?.toJSON()).toStrictEqual(bs);

            if (bs.length <= 4 || Math.random() >= 0.5) {
                let val = ((Math.random() * 256) | 0).toString(16);

                val = val.length == 1 ? "0" + val : val;

                var gasLimit = await weight(conn, contract, "push", ["0x" + val]);
                const tx = contract.tx.push({ gasLimit }, ["0x" + val]);

                await transaction(tx, alice);

                bs += val;
            } else {
                const tx = contract.tx.pop({ gasLimit });

                await transaction(tx, alice);

                // note that substrate does not give us access to the return values of a transaction;
                // therefore, we can't check the return values of pop

                bs = bs.slice(0, -2);
            }

        }
    });
});
