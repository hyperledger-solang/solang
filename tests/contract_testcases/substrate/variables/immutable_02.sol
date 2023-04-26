contract x {
            int public immutable y = 1;

            function foo() public {
                y++;
            }
        }
        
// ----
// error (106-107): cannot assign to immutable outside of constructor
