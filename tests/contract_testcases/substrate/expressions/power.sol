contract test {
            function power(uint64 base, int64 exp) public returns (uint64) {
                return base ** exp;
            }
       }
// ----
// error (116-127): exponation (**) is not allowed with signed types
