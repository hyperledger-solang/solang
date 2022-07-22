import expect from 'expect';
import { gasLimit, createConnection, deploy, transaction, aliceKeypair, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';

describe('issue666 flip and inc', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('tests for issue #666', async function () {
        this.timeout(50000);

        const alice = aliceKeypair();

        // call the constructors
        let flipper_contract = await deploy(conn, alice, 'Flip.contract', BigInt(0));
        let inc_contract = await deploy(conn, alice, 'Inc.contract', BigInt(0), flipper_contract.address);

        try {
            // This works for ink contracts and should work for solang compiled contracts too (issue 666):
            let ss58_addr = flipper_contract.address.toString();
            await deploy(conn, alice, 'Inc.contract', BigInt(0), ss58_addr);
            expect(false).toBeTruthy();
        }
        catch (satan) {
            expect(satan).toStrictEqual(Error('createType(AccountId):: Expected input with 32 bytes (256 bits), found 48 bytes'));
        }

        let contract = new ContractPromise(conn, inc_contract.abi, inc_contract.address);

        let tx = contract.tx.superFlip({ gasLimit });
    });
});
