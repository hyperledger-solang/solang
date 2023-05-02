
        contract bar {
            function test() public {
                int64 b = tx.origin;

                assert(b == 93_603_701_976_053);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:27-29: builtin 'tx.origin' does not exist
