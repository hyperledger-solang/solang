
        contract tester {
            function test(bytes bs) public returns (bytes32) {
                bytes32 hash = keccak256(bs);

                return hash;
            }
        }