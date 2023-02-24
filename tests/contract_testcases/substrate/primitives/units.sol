
        contract c {
            function test() public {
                int64 x = 1 ether;
                int64 wei = 1 sol;
                // dot is not a unit, produce an error!
                int64 dot = 1 dot;
            }
        }