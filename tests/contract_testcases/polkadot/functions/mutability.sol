contract test {
            int64 foo = 1844674;

            function bar() public pure returns (int64) {
                return foo;
            }
        }
// ---- Expect: diagnostics ----
// error: 5:24-27: function declared 'pure' but this expression reads from state
