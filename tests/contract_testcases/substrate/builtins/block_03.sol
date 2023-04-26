
        contract bar {
            function test() public {
                int64 b = block.coinbase;

                assert(b == 93_603_701_976_053);
            }
        }
// ----
// error (87-92): builtin 'block.coinbase' does not exist
