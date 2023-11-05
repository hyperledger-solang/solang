
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo ({a:true, b:"ab"});
            }
        }
// ---- Expect: diagnostics ----
// error: 5:17-44: event cannot be emitted with named fields as 2 of its fields do not have names
// 	note 3:19-22: definition of foo
