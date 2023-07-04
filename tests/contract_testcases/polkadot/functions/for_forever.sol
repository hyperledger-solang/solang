
    contract test {
        function goodset() public returns (bool) {
            for (;;) {
                // ...
            }
            return;
        }
    }
// ---- Expect: diagnostics ----
// error: 7:13-19: unreachable statement
