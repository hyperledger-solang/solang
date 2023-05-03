abstract contract test {
            function bar(int[] storage foo) internal view {
                foo[0] = 102;
            }
        }
// ---- Expect: diagnostics ----
// warning: 2:40-43: function parameter 'foo' has never been read
// error: 3:17-23: function declared 'view' but this expression writes to state
