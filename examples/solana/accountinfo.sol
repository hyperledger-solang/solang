import {AccountInfo} from "solana";

contract SplToken {
    function get_token_account(address token)
        internal
        view
        returns (AccountInfo)
    {
        for (uint64 i = 0; i < tx.accounts.length; i++) {
            AccountInfo ai = tx.accounts[i];
            if (ai.key == token) {
                return ai;
            }
        }

        revert("token not found");
    }

    function total_supply(address token) public view returns (uint64) {
        AccountInfo account = get_token_account(token);

        return account.data.readUint64LE(33);
    }
}
