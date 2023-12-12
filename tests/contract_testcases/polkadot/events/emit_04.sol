
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo ({a:true, a:"ab"});
            }
        }
// ---- Expect: diagnostics ----
// error: 5:36-37: duplicate argument with name 'a'
