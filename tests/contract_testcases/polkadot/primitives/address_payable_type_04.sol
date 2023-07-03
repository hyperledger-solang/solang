
        contract c {
            function test(address payable a) public {
                other b = other(a);
            }
        }

        contract other {
            function test() public {
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-52: function can be declared 'pure'
// warning: 4:23-24: local variable 'b' has been assigned, but never read
// warning: 9:13-35: function can be declared 'pure'
