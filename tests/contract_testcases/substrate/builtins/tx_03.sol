
        contract bar {
            function test() public {
                int64 b = tx.origin;

                assert(b == 93_603_701_976_053);
            }
        }