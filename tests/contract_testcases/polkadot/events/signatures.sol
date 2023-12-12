
        event foo(bool a, int b);
        event bar(bool a, int b);

        contract c {
            event foo(int b);
            event bar(int b);

            function f() public {
                emit foo(true, 1);
            }
        }

// ---- Expect: diagnostics ----
// warning: 3:15-18: event 'bar' has never been emitted
// warning: 6:19-22: event 'foo' has never been emitted
// warning: 7:19-22: event 'bar' has never been emitted
