
        contract tester {
            function test(bytes bs) public returns (bytes32) {
                bytes32 hash = sha256(bs);

                return hash;
            }
        }