contract Test1 {
    @account(foo)
    @mutableAccount(bar)
    @signer(signerFoo)
    @mutableSigner(signerBar)
    function doThis() external returns (uint64) {
        return 64;
    }
}

// ---- Expect: diagnostics ----
// error: 2:5-18: unknown annotation account for function
// error: 3:5-25: unknown annotation mutableAccount for function
// error: 4:5-23: unknown annotation signer for function
// error: 5:5-30: unknown annotation mutableSigner for function