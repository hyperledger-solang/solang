
        contract bar {
            function test() public {
                int64 b = tx.gasprice(100);

                assert(b == 14_250_083_331_950_119_597);
            }
        }
// ----
// error (87-103): implicit conversion would change sign from uint128 to int64
