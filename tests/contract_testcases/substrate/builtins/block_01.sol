
        contract bar {
            function test() public {
                int64 b = block.timestamp;

                assert(b == 14_250_083_331_950_119_597);
            }
        }