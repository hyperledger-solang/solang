contract test {
            function power(int64 base, int64 exp) public returns (int64) {
                return base ** exp;
            }
       }
// ----
// error (114-125): exponation (**) is not allowed with signed types
