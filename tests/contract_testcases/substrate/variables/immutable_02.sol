contract x {
            int public immutable y = 1;

            function foo() public {
                y++;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 5:17-18: cannot assign to immutable outside of constructor
