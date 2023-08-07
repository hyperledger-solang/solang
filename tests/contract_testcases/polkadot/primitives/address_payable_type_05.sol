
        contract c {
            function test(address payable a) public {
                address b = address(a);
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-52: function can be declared 'pure'
// warning: 4:25-26: local variable 'b' is unused
