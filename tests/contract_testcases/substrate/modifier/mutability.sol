
        contract c {
            uint64 var;
            modifier foo(uint64 x) { _; }

            function bar() foo(var) public pure {}
        }
// ----
// warning (78-79): function parameter 'x' has never been read
// error (120-123): function declared 'pure' but this expression reads from state
