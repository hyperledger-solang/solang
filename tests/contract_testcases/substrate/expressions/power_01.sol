contract test {
            function power(int64 base, uint64 exp) public returns (int64) {
                return base ** exp;
            }
       }
// ---- Expect: diagnostics ----
// error: 3:24-35: exponation (**) is not allowed with signed types
