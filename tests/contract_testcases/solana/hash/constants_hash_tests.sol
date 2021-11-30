
        contract tester {
            function test() public {
                bytes32 hash = blake2_256("Hello, World!");

                assert(hash == hex"527a6a4b9a6da75607546842e0e00105350b1aaf");
            }
        }