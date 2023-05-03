
        contract test {
            using ints for uint64;
            function foo(uint32 x) public pure returns (uint64) {
                // x is 32 bit but the max function takes 64 bit uint
                return x.max(65536);
            }
        }

        library ints {
            function max(uint64 a, uint64 b) internal pure returns (uint64) {
                return a > b ? a : b;
            }
        }
// ---- Expect: diagnostics ----
