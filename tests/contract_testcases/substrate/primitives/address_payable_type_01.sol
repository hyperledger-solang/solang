
        contract c {
            function test1(address payable a, address b) public returns (bool) {
                return a == b;
            }

            function test2(address payable a, address b) public returns (bool) {
                return b == a;
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-79: function can be declared 'pure'
// warning: 7:13-79: function can be declared 'pure'
