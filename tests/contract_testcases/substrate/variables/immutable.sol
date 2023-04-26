contract x {
            int public immutable y = 1;

            function foo() public {
                y = 2;
            }
        }
        
// ----
// error (106-107): cannot assign to immutable 'y' outside of constructor
