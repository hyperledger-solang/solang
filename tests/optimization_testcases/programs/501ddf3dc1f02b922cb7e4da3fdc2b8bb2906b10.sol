
        contract tester {
            function test(bytes bs) public returns (bytes20) {
                bytes20 hash = ripemd160(bs);

                return hash;
            }
        }