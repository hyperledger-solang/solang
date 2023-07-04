
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo ({a:true, a:"ab"});
            }
        }
// ---- Expect: diagnostics ----
// error: 5:17-44: event cannot be emmited with named fields as 2 of its fields do not have names
// 	note 3:19-22: definition of foo
// error: 5:36-37: duplicate argument with name 'a'
