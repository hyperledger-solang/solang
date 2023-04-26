
        contract b {
            function step1() public returns (address) {
                return this;
            }
        }
// ----
// error (94-105): implicit conversion to address from contract b not allowed
