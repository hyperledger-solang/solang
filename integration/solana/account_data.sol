import '../../solana-library/spl_token.sol';

contract AccountData {
    @account(addr)
    function token_account() view external returns (SplToken.TokenAccountData) {
        return SplToken.get_token_account_data(tx.accounts.addr);
    }

    @account(addr)
    function mint_account() view external returns (SplToken.MintAccountData) {
        return SplToken.get_mint_account_data(tx.accounts.addr);
    }
}