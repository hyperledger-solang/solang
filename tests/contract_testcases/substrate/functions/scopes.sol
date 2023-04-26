
    contract test {
        function goodset() public returns (bool) {
            {
                bool a = true;
            }
            return a;
        }
    }
// ----
// error (150-151): 'a' not found
