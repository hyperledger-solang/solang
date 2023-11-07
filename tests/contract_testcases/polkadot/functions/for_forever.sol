
    contract test {
        function goodset() public returns (bool) {
            for (;;) {
                // ...
            }
            return;
        }
    }
// ---- Expect: diagnostics ----
// warning: 3:9-49: function can be declared 'pure'
// warning: 7:13-19: unreachable statement
