contract test3 {
            // The resolver should figure out how many breaks there
            // in the for loop; if there are none, then the basic block
            // after the loop need not be created
            function halting_problem() public returns (uint32) {
                for (;;) {
                }
                return 0;
            }
        }
// ----
// error (333-341): unreachable statement
