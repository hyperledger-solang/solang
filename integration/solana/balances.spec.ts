import expect from 'expect';
import { loadContract } from './utils';

describe('Deploy solang contract and test', () => {
    it('balances', async function () {
        this.timeout(50000);

        let [token, connection, payerAccount] = await loadContract('balances', 'balances.abi');

        let payer = '0x' + payerAccount.publicKey.toBuffer().toString('hex');

        let res = await token.functions.get_balance(payer, {
            accounts: [payerAccount.publicKey],
        });

        let bal = Number(res.result[0]);

        let rpc_bal = await connection.getBalance(payerAccount.publicKey);

        expect(bal + 5000).toBe(rpc_bal);

        // @solana/solidity needs a fix for this
        // res = await token.functions.pay_me({
        //     value: 1000,
        //     writableAccounts: [payerAccount.publicKey],
        // });

        // expect(res.log).toContain('Thank you very much for 1000');

        // expect(await connection.getBalance(token.storageAccount)).toBe(1000);
    });
});
