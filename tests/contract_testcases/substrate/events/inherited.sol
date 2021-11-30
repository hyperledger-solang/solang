
        contract base {
            event foo(bool a, int b);
        }

        contract c is base {
            function f() public {
                emit foo(true, 1);
            }
        }