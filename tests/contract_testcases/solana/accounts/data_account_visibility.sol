contract Foo {
    function addr_account() public pure returns (address) {
        return tx.accounts.dataAccount.key;
    }
}

// ---- Expect: diagnostics ----
// error: 3:16-27: function declared 'pure' but this expression reads from state