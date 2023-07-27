
        contract c {
            function foo() public {
                    bytes f = bytes(new string(2));
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-34: function can be declared 'pure'
// warning: 4:27-28: local variable 'f' is unused
