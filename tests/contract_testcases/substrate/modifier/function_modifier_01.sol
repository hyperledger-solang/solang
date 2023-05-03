
        contract c {
            modifier foo() { _; }

            function bar() foo(1) public {}
        }
// ---- Expect: diagnostics ----
// error: 5:28-34: modifier expects 0 arguments, 1 provided
