
        contract c {
            event foo(bool,uint32);
            function f() view public {
                emit foo (true, 102);
            }
        }
// ---- Expect: diagnostics ----
// error: 5:17-37: function declared 'view' but this expression writes to state
