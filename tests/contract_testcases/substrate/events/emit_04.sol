
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo ({a:true, a:"ab"});
            }
        }