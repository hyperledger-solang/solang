import expect from 'expect';
import crypto from 'crypto';
import { createConnection, deploy, transaction, aliceKeypair, weight, query, } from './index';
import { ContractPromise } from '@polkadot/api-contract';
import { ApiPromise } from '@polkadot/api';

describe('Deploy arrays contract and test', () => {
    let conn: ApiPromise;

    before(async function () {
        conn = await createConnection();
    });

    after(async function () {
        await conn.disconnect();
    });

    it('arrays in account storage', async function () {
        this.timeout(50000);

        const alice = aliceKeypair();

        let deployed_contract = await deploy(conn, alice, 'arrays.contract', BigInt(0));

        let contract = new ContractPromise(conn, deployed_contract.abi, deployed_contract.address);

        let users = [];

        for (let i = 0; i < 3; i++) {
            let addr = '0x' + crypto.randomBytes(32).toString('hex');
            let name = `name${i}`;
            let id = crypto.randomBytes(4).readUInt32BE(0) % 1024;
            let perms: string[] = [];

            for (let j = 0; j < Math.random() * 3; j++) {
                let p = Math.floor(Math.random() * 8);

                perms.push(`Perm${p + 1}`);
            }

            let gasLimit = await weight(conn, contract, "addUser", [id, addr, name, perms]);
            const tx1 = contract.tx.addUser({ gasLimit }, id, addr, name, perms);

            await transaction(tx1, alice);

            users.push({ "name": name, "addr": addr, "id": id, "perms": perms });
        }

        let user = users[Math.floor(Math.random() * users.length)];

        let res1 = await query(conn, alice, contract, "getUserById", [user.id]);

        expect(res1.output?.toJSON()).toStrictEqual(user);

        if (user.perms.length > 0) {
            let perms = user.perms;

            let p = perms[Math.floor(Math.random() * perms.length)];

            let res2 = await query(conn, alice, contract, "hasPermission", [user.id, p]);

            expect(res2.output?.toJSON()).toBe(true);
        }

        user = users[Math.floor(Math.random() * users.length)];

        let res3 = await query(conn, alice, contract, "getUserByAddress", [user.addr]);

        expect(res3.output?.toJSON()).toStrictEqual(user);

        let gasLimit = await weight(conn, contract, "removeUser", [user.id]);
        const tx2 = contract.tx.removeUser({ gasLimit }, user.id);

        await transaction(tx2, alice);

        let res4 = await query(conn, alice, contract, "userExists", [user.id]);

        expect(res4.output?.toJSON()).toBe(false);
    });
});
