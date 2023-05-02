
        event foo(bool a, int b);

        contract c {
            event foo(int b);
            event foo(int x);

            function f() public {
                emit foo(true, 1);
            }
        }
// ---- Expect: diagnostics ----
