
        contract c {
            function test1(address payable a, address b) public returns (bool) {
                return a == b;
            }

            function test2(address payable a, address b) public returns (bool) {
                return b == a;
            }
        }
// ----
// warning (34-100): function can be declared 'pure'
// warning (161-227): function can be declared 'pure'
