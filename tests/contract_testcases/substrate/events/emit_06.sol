
        contract c {
            event foo(bool,uint32);
            function f() view public {
                emit foo (true, 102);
            }
        }