
        contract bar {
            function test() public {
                int64 b = block.gaslimit;

                assert(b == 93_603_701_976_053);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:27-32: builtin 'block.gaslimit' does not exist
