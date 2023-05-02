contract test {
            int64 foo = 1844674;

            function bar() public view {
                foo = 102;
            }
        }
// ---- Expect: diagnostics ----
// error: 5:17-20: function declared 'view' but this expression writes to state
