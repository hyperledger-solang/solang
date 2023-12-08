
        event foo(bool a, int b);
        event foo(bool x, int y);

        contract c {
            event foo(int b);

            function f() public {
                emit foo(true, 1);
            }
        }

// ---- Expect: diagnostics ----
// warning: 3:15-18: event 'foo' has never been emitted
// warning: 6:19-22: event 'foo' has never been emitted
