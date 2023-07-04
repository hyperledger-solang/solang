import expect from 'expect';
import { weight, createConnection, deploy, aliceKeypair, transaction, query } from './index';
import { ContractPromise } from '@polkadot/api-contract';

describe('Deploy array_struct_mapping_storage contract and test', () => {
    it('array_struct_mapping_storage', async function () {
        this.timeout(200000);

        let conn = await createConnection();
        const alice = aliceKeypair();

        let deployed_contract = await deploy(conn, alice, 'array_struct_mapping_storage.contract', BigInt(0));

        let contract = new ContractPromise(conn, deployed_contract.abi, deployed_contract.address);

        // first set a canary
        let gasLimit = await weight(conn, contract, "setNumber", [2147483647]);
        let tx = contract.tx.setNumber({ gasLimit }, 2147483647);

        await transaction(tx, alice);

        // let's add two elements to our array
        tx = contract.tx.push({ gasLimit });

        await transaction(tx, alice);

        tx = contract.tx.push({ gasLimit });

        await transaction(tx, alice);

        // set some values
        for (let array_no = 0; array_no < 2; array_no += 1) {
            for (let i = 0; i < 10; i += 1) {
                tx = contract.tx.set({ gasLimit }, array_no, 102 + i + array_no * 500, 300331 + i);

                await transaction(tx, alice);
            }
        }

        // test our values
        for (let array_no = 0; array_no < 2; array_no += 1) {
            for (let i = 0; i < 10; i += 1) {
                let { output } = await query(conn, alice, contract, "get", [array_no, 102 + i + array_no * 500]);

                let number = Number.parseInt(output!.toString());

                expect(number).toEqual(300331 + i);
            }
        }

        // delete one and try again
        tx = contract.tx.rm({ gasLimit }, 0, 104);

        await transaction(tx, alice);

        for (let i = 0; i < 10; i += 1) {
            let { output } = await query(conn, alice, contract, "get", [0, 102 + i]);

            let number = Number.parseInt(output!.toString());

            if (i != 2) {
                expect(number).toEqual(300331 + i);
            } else {
                expect(number).toEqual(0);
            }
        }

        // test our canary
        let { output } = await query(conn, alice, contract, "number");

        let number = Number.parseInt(output!.toString());

        expect(number).toEqual(2147483647);

        conn.disconnect();
    });
});
