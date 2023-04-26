contract test {
            function bar() public pure return (int64) {
                return 1;
            }
        }
// ----
// error (55-61): 'return' unexpected. Did you mean 'returns'?
