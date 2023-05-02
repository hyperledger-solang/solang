
        contract c {
            event foo(bool,uint32);
            function f() public {
                emit foo (true, "ab");
            }
        }
// ---- Expect: diagnostics ----
// error: 5:33-37: implicit conversion to uint32 from bytes2 not allowed
