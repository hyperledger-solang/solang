
        contract printer {
            function test() public {
                printer x = printer(address(102));
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-35: function can be declared 'pure'
// warning: 4:25-26: local variable 'x' is unused
