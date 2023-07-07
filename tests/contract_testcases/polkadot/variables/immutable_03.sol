contract x {
            int[] public immutable y;

            function foo() public {
                y.push();
            }
        }
        
// ---- Expect: diagnostics ----
// error: 5:17-25: cannot call method on immutable array outside of constructor
