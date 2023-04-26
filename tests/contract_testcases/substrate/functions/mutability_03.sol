contract test {
            int64 foo = 1844674;

            function bar() public view {
                foo = 102;
            }
        }
// ----
// error (107-110): function declared 'view' but this expression writes to state
