
    contract test {
        function goodset() public returns (uint) {
            for (uint i = 0; i < 10 ; i++) {
                bool a = true;
            }
            return i;
        }
    }
// ---- Expect: diagnostics ----
// error: 7:20-21: 'i' not found
