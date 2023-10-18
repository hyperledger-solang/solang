contract Test1 {
    @account(foo)
    @mutableAccount(bar)
    @signer(signerFoo)
    @mutableSigner(signerBar)
    function doThis() external returns (uint64) {
        assert(tx.accounts.signerFoo.is_signer);
        assert(tx.accounts.signerBar.is_signer);

        return tx.accounts.foo.lamports;
    }
}

contract Test2 {
    @account(t1Id)
    @account(foo)
    function callThat() external returns (uint64) {
        uint64 res = Test1.doThis{program_id: tx.accounts.t1Id.key}();
        return res;
    }
}

// ---- Expect: diagnostics ----
// warning: 6:5-48: function can be declared 'view'
// error: 16:5-18: account name collision encountered. Calling a function that requires an account whose name is also defined in the current function will create duplicate names in the IDL. Please, rename one of the accounts
// 	note 2:5-18: other declaration