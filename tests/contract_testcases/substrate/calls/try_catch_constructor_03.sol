
        contract c {
            function f() public {
                x : 1
            }
        }
// ---- Expect: diagnostics ----
// error: 3:33-5:14: expected code block, not list of named arguments
