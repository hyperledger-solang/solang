
        contract bar {
            function test() public {
                int64 b = gasleft(1);

                assert(b == 14_250_083_331_950_119_597);
            }
        }
// ----
// error (87-94): builtin function 'gasleft' expects 0 arguments, 1 provided
