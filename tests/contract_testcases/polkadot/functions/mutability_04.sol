abstract contract test {
            function bar(int[] storage foo) internal view {
                foo[0] = 102;
            }
        }
// ---- Expect: diagnostics ----
// error: 3:17-23: function declared 'view' but this expression writes to state
