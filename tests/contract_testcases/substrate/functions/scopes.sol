
    contract test {
        function goodset() public returns (bool) {
            {
                bool a = true;
            }
            return a;
        }
    }
// ---- Expect: diagnostics ----
// error: 7:20-21: 'a' not found
