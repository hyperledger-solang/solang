
        contract c {
            uint64 var;
            modifier foo() { uint64 x = var; _; }

            function bar() foo() public pure {}
        }
// ----
// warning (82-83): local variable 'x' has been assigned, but never read
// error (86-89): function declared 'pure' but this expression reads from state
