contract test {
            function bar() public pure return (int64) {
                return 1;
            }
        }
// ---- Expect: diagnostics ----
// error: 2:40-46: 'return' unexpected. Did you mean 'returns'?
