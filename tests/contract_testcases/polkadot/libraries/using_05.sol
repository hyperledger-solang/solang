
        contract test {
            using ints for uint32;
            function foo(uint32 x) public pure returns (uint64) {
                // x is 32 bit but the max function takes 64 bit uint
                return x.max(65536, 2);
            }
        }

        library ints {
            function max(uint64 a, uint64 b) internal pure returns (uint64) {
                return a > b ? a : b;
            }
        }
// ---- Expect: diagnostics ----
// error: 6:24-39: using function expects 2 arguments, 3 provided (including self)
