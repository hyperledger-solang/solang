
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo (true, "ab");
            }
        }
// ----
// error (124-128): implicit conversion to uint32 from bytes2 not allowed
