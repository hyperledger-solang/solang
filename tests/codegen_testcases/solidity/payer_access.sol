// RUN: --target solana --emit cfg

@program_id("Seed23VDZ9HFCfKvFwmemB6dpi25n5XjZdP52B2RUmh")
contract Seed2 {

    @payer(payer)
    @seed("sunflower")
    @space(23)
    // BEGIN-CHECK: Seed2::Seed2::constructor
    constructor(@seed bytes ss) {
        // CHECK: ty:struct AccountInfo %temp.1 = (subscript struct AccountInfo[] (builtin Accounts ())[uint32 1])
        // CHECK: (load (load (struct %temp.1 field 0)))
        assert(tx.accounts.payer.key == address(this));
        // CHECK: AccountInfo %temp.2 = (subscript struct AccountInfo[] (builtin Accounts ())[uint32 1])
        // CHECK: (load (struct %temp.2 field 5))
        assert(tx.accounts.payer.is_signer);
        // CHECK: AccountInfo %temp.3 = (subscript struct AccountInfo[] (builtin Accounts ())[uint32 1])
        // CHECK: (load (struct %temp.3 field 6))
        assert(tx.accounts.payer.is_writable);
        print("In Seed2 constructor");
    }

}