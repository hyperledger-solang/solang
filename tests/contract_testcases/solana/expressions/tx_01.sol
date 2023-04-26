
        contract foo {
            function test() public pure returns (uint64) {
                return tx.gasprice;
            }
        }
// ----
// error (106-108): builtin 'tx.gasprice' does not exist
