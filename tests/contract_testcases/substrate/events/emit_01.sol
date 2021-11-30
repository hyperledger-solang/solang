
        contract c {
            event foo(bool);
            function f() public {
                emit foo {};
            }
        }