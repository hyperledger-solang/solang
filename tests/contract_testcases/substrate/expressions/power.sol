contract test {
            function power(uint64 base, int64 exp) public returns (uint64) {
                return base ** exp;
            }
       }
// ---- Expect: diagnostics ----
// error: 3:24-35: exponation (**) is not allowed with signed types
