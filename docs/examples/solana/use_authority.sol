import 'solana';

contract AuthorityExample {
    address authority;
    uint64 counter;

    constructor(address initial_authority) {
        authority = initial_authority;
    }

    @signer(authorityAccount)
    function set_new_authority(address new_authority) external {
        assert(tx.accounts.authorityAccount.key == authority && tx.accounts.authorityAccount.is_signer);
        authority = new_authority;
    }

    @signer(authorityAccount)
    function inc() external {
        assert(tx.accounts.authorityAccount.key == authority && tx.accounts.authorityAccount.is_signer);
        counter += 1;
    }

    function get() public view returns (uint64) {
        return counter;
    }
}