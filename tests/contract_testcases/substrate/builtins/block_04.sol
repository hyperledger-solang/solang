
        contract bar {
            function test() public {
                int64 b = block.gaslimit;

                assert(b == 93_603_701_976_053);
            }
        }
// ----
// error (87-92): builtin 'block.gaslimit' does not exist
