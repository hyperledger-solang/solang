contract x {
            int public immutable y = 1;

            function foo() public {
                y = 2;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 5:17-18: cannot assign to immutable 'y' outside of constructor
