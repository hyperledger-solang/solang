import '../../solana-library/spl_token.sol';

contract Token {
    address mint;

    function set_mint(address _mint) public {
        mint = _mint;
    }

    @account(mint)
    function total_supply() external view returns (uint64) {
        assert(tx.accounts.mint.key == mint);
        return SplToken.total_supply(tx.accounts.mint);
    }

    @account(account)
    function get_balance() external view returns (uint64) {
        return SplToken.get_balance(tx.accounts.account);
    }

    @mutableAccount(mint)
    @mutableAccount(account)
    @signer(authority)
    function mint_to(uint64 amount) external {
        assert(tx.accounts.mint.key == mint);
        SplToken.mint_to(
            tx.accounts.mint.key, 
            tx.accounts.account.key, 
            tx.accounts.authority.key, 
            amount);
    }

    @mutableAccount(from)
    @mutableAccount(to)
    @signer(owner)
    function transfer(uint64 amount) external {
        SplToken.transfer(
            tx.accounts.from.key, 
            tx.accounts.to.key, 
            tx.accounts.owner.key,
            amount);
    }

    @mutableAccount(account)
    @mutableAccount(mint)
    @signer(owner)
    function burn(uint64 amount) external {
        SplToken.burn(
            tx.accounts.account.key, 
            tx.accounts.mint.key, 
            tx.accounts.owner.key,
            amount);
    }
}