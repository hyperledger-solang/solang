import '../../solana-library/spl_token.sol';

contract AccountData {
    function token_account(address addr) view public returns (SplToken.TokenAccountData) {
        return SplToken.get_token_account_data(addr);
    }

    function mint_account(address addr) view public returns (SplToken.MintAccountData) {
        return SplToken.get_mint_account_data(addr);
    }
}