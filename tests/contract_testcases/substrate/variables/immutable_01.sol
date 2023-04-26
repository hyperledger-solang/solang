contract x {
            int public immutable y = 1;

            function foo() public {
                y += 1;
            }
        }
        
// ----
// error (106-112): cannot assign to immutable outside of constructor
