
        event foo(bool a, int b);
        event bar(bool a, int b);

        contract c {
            event foo(int b);
            event bar(int b);

            function f() public {
                emit foo(true, 1);
            }
        }