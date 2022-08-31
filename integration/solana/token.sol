import '../../solana-library/spl_token.sol';

contract Token {
    address mint;

    function set_mint(address _mint) public {
        mint = _mint;
    }

    function total_supply() public view returns (uint64) {
        return SplToken.total_supply(mint);
    }

    function get_balance(address account) public view returns (uint64) {
        return SplToken.get_balance(account);
    }

    function mint_to(address account, address authority, uint64 amount) public {
        SplToken.mint_to(mint, account, authority, amount);
    }

    function transfer(address from, address to, address owner, uint64 amount) public {
        SplToken.transfer(from, to, owner, amount);
    }

    function burn(address account, address owner, uint64 amount) public {
        SplToken.burn(account, mint, owner, amount);
    }
}