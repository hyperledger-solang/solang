contract x {
            int[] public immutable y;

            function foo() public {
                y.push();
            }
        }
        
// ----
// error (104-112): cannot call method on immutable array outside of constructor
