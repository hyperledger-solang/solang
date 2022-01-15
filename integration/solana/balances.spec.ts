import expect from 'expect';
import { publicKeyToHex } from '@solana/solidity';
import { loadContract } from './setup';

describe('Deploy solang contract and test', function () {
    this.timeout(500000);

    it('balances', async function () {
        let { contract, connection, payer } = await loadContract('balances', 'balances.abi');

        let res = await contract.functions.get_balance(publicKeyToHex(payer.publicKey), {
            accounts: [payer.publicKey],
        });

        let bal = Number(res.result);

        let rpc_bal = await connection.getBalance(payer.publicKey);

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
