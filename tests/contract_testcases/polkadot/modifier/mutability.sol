
        contract c {
            uint64 var;
            modifier foo(uint64 x) { _; }

            function bar() foo(var) public pure {}
        }
// ---- Expect: diagnostics ----
// warning: 4:33-34: function parameter 'x' is unused
// error: 6:32-35: function declared 'pure' but this expression reads from state
