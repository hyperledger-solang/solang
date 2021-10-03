import expect from 'expect';
import { loadContract } from './utils';
import { ContractFunctionCallOptions } from '@solana/solidity';

describe('Deploy solang contract and test', () => {
    it('balances', async function () {
        this.timeout(50000);

        let [token, connection, payerAccount] = await loadContract('balances', 'balances.abi');

        let payer = '0x' + payerAccount.publicKey.toBuffer().toString('hex');

        let options: ContractFunctionCallOptions = {
            accounts: [payerAccount.publicKey],
        };

        let res = await token.functions.get_balance(payer, options);

        let bal = Number(res.result);

        let rpc_bal = await connection.getBalance(payerAccount.publicKey);

        expect(bal).toBe(rpc_bal);
    });
});
