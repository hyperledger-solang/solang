
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo (true, "ab");
            }
        }