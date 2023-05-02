contract x {
            int public immutable y;

            function foo() public {
                int a;

                (y, a) = (1, 2);
            }
        }
        
// ---- Expect: diagnostics ----
// error: 7:18-19: cannot assign to immutable 'y' outside of constructor
