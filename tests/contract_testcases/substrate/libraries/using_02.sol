
        library x {
            function max(uint64 a, uint64 b) private pure returns (uint64) {
                return a > b ? a : b;
            }
        }

        contract c {
            using x for asdf;
        }
// ----
// error (206-210): type 'asdf' not found
