
        contract foo {
            function test() public pure returns (address) {
                return tx.origin;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:24-26: builtin 'tx.origin' does not exist
