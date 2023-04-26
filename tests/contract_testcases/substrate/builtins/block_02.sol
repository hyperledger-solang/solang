
        contract bar {
            function test() public {
                int64 b = block.minimum_balance;

                assert(b == 93_603_701_976_053);
            }
        }
// ----
// error (87-108): implicit conversion would change sign from uint128 to int64
