contract x {
            int public immutable y;

            function foo() public {
                int a;

                (y, a) = (1, 2);
            }
        }
        
// ----
// error (127-128): cannot assign to immutable 'y' outside of constructor
