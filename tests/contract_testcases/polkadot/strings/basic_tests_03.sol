
        contract c {
            function foo() public {
                    string f = string(new bytes(2));
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-34: function can be declared 'pure'
// warning: 4:28-29: local variable 'f' is unused
