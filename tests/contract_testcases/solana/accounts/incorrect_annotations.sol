contract Test1 {

    uint32 g;
    @account(foo)
    @mutableAccount(bar)
    function doThis() public returns (uint64) {

        return tx.accounts.foo.lamports;
    }

    @account(32)
    @signer("Hello")
    function invalid_paramter() external view returns (uint32) {
        return g;
    }

    @bar(foo)
    function invalid_annotation() public view returns (uint32) {
        return g;
    }
}

// ---- Expect: diagnostics ----
// error: 4:5-18: account declarations are only valid in functions declared as external
// error: 5:5-25: account declarations are only valid in functions declared as external
// error: 8:28-31: unrecognized account
// error: 11:5-17: invalid parameter for annotation
// error: 12:5-21: invalid parameter for annotation
// error: 17:5-14: unknown annotation bar for function