
        event foo(bool a, int b);
        event foo(bool x, int y);

        contract c {
            event foo(int b);

            function f() public {
                emit foo(true, 1);
            }
        }
// ----
// warning (49-52): event 'foo' has never been emitted
