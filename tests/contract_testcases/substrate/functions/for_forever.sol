
    contract test {
        function goodset() public returns (bool) {
            for (;;) {
                // ...
            }
            return;
        }
    }
// ----
// error (144-150): unreachable statement
