
    contract test {
        function goodset() public returns (uint) {
            for (uint i = 0; i < 10 ; i++) {
                bool a = true;
            }
            return i;
        }
    }
// ----
// error (181-182): 'i' not found
