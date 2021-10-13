import expect from 'expect';
import { establishConnection } from './index';

describe('Deploy solang contract and test', () => {
    it('balances', async function () {
        this.timeout(50000);

        let conn = await establishConnection();

        let hash_functions = await conn.loadProgram("bundle.so", "balances.abi");

        // call the constructor
        await hash_functions.call_constructor(conn, 'balances', []);

        let payer = '0x' + conn.payerAccount.publicKey.toBuffer().toString('hex');

        let res = await hash_functions.call_function(conn, "get_balance", [payer], [conn.payerAccount.publicKey]);
        let bal = Number(res[0]);

        let rpc_bal = await conn.connection.getBalance(conn.payerAccount.publicKey);

        console.log("bal from rpc " + bal);

        expect(bal).toBe(rpc_bal);
    });
});
