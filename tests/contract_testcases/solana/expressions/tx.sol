
        contract foo {
            function test() public pure returns (address) {
                return tx.origin;
            }
        }
// ----
// error (107-109): builtin 'tx.origin' does not exist
