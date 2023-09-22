abstract contract test {
            function bar(int64[] storage foo) private pure returns (int64) {
                return foo[0];
            }
        }
// ---- Expect: diagnostics ----
// error: 3:24-30: function declared 'pure' but this expression reads from state
