
        contract b {
            function step1() public returns (address) {
                return this;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:24-28: implicit conversion to address from contract b not allowed
