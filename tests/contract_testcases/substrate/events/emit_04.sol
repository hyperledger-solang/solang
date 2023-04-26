
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo ({a:true, a:"ab"});
            }
        }
// ----
// error (108-135): event cannot be emmited with named fields as 2 of its fields do not have names
// 	note (40-43): definition of foo
// error (127-128): duplicate argument with name 'a'
