
        contract c {
            uint64 var;
            modifier foo() { uint64 x = var; _; }

            function bar() foo() public pure {}
        }
// ---- Expect: diagnostics ----
// warning: 4:37-38: local variable 'x' has been assigned, but never read
// error: 4:41-44: function declared 'pure' but this expression reads from state
