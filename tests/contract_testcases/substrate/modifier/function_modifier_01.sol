
        contract c {
            modifier foo() { _; }

            function bar() foo(1) public {}
        }
// ----
// error (84-90): modifier expects 0 arguments, 1 provided
