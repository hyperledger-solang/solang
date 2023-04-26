
        contract bar {
            function test() public {
                int64 b = block.difficulty;

                assert(b == 93_603_701_976_053);
            }
        }
// ----
// error (87-92): builtin 'block.difficulty' does not exist
