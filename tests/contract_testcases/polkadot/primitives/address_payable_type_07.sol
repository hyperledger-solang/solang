
        contract c {
            function test(address a) public {
                address payable b = address payable(a);
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-44: function can be declared 'pure'
// warning: 4:33-34: local variable 'b' is unused
