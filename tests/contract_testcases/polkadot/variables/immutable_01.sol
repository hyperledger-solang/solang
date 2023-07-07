contract x {
            int public immutable y = 1;

            function foo() public {
                y += 1;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 5:17-23: cannot assign to immutable outside of constructor
