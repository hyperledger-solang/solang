
        event foo(bool a, int b);

        contract c {
            event foo(bool x, int y);

            function f() public {
                emit foo(true, 1);
            }
        }
// ---- Expect: diagnostics ----
