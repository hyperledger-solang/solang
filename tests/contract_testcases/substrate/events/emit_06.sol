
        contract c {
            event foo(bool,uint32);
            function f() view public {
                emit foo (true, 102);
            }
        }
// ----
// error (113-133): function declared 'view' but this expression writes to state
