
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo (true);
            }
        }
// ---- Expect: diagnostics ----
// error: 5:17-32: event type 'foo' has 2 fields, 1 provided
// 	note 3:19-22: definition of foo
