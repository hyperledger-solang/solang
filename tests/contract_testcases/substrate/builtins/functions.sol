
        contract bar {
            function test() public {
                int64 b = gasleft();

                assert(b == 14_250_083_331_950_119_597);
            }
        }
// ----
// error (87-94): implicit conversion would change sign from uint64 to int64
