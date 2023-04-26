
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo (true);
            }
        }
// ----
// error (108-123): event type 'foo' has 2 fields, 1 provided
