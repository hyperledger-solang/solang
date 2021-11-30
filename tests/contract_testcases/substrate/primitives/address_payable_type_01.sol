
        contract c {
            function test1(address payable a, address b) public returns (bool) {
                return a == b;
            }

            function test2(address payable a, address b) public returns (bool) {
                return b == a;
            }
        }