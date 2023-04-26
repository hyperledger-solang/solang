
        event foo(bool a, int b);
        event bar(bool a, int b);

        contract c {
            event foo(int b);
            event bar(int b);

            function f() public {
                emit foo(true, 1);
            }
        }
// ----
// warning (49-52): event 'bar' has never been emitted
// warning (139-142): event 'bar' has never been emitted
