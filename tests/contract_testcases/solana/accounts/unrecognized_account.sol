@program_id("Seed23VDZ9HFCfKvFwmemB6dpi25n5XjZdP52B2RUmh")
contract Seed2 {
    bytes my_seed;

    @payer(payer)
    @seed("sunflower")
    @space(23)
    constructor(@seed bytes ss) {
        my_seed = ss;
        assert(tx.accounts.payer.key == address(this));
        assert(tx.accounts.other_account.key == address(this));
        print("In Seed2 constructor");
    }

    function foo() public returns (address) {
        return tx.accounts.my_account.key;
    }

}

contract Other {
    constructor(address acc) {
        assert(tx.accounts.payer.key == address(this));
    }
}

// ---- Expect: diagnostics ----
// error: 11:28-41: unrecognized account
// error: 16:28-38: unrecognized account
// error: 23:28-33: unrecognized account