
        contract bar {
            function test() public {
                int64 b = block.timestamp;

                assert(b == 14_250_083_331_950_119_597);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:27-42: implicit conversion would change sign from uint64 to int64
