
        contract foo {
            function test() public pure returns (uint64) {
                return tx.gasprice;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:24-26: builtin 'tx.gasprice' does not exist
