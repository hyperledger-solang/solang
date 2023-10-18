contract Test0 {
    @account(foo)
    @mutableAccount(foo)
    @signer(signerFoo)
    @mutableSigner(signerFoo)
    function doThis() external returns (uint64) {
        return 64;
    }
}

// ---- Expect: diagnostics ----
// error: 3:21-24: account 'foo' already defined
// 	note 2:5-18: previous definition
// error: 5:20-29: account 'signerFoo' already defined
// 	note 4:5-23: previous definition
