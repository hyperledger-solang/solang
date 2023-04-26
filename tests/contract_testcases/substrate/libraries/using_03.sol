
        library x {
            function max(uint64 a, uint64 b) private pure returns (uint64) {
                return a > b ? a : b;
            }
        }

        contract c {
            using x for x;
        }
// ----
// error (206-207): using for library 'x' type not permitted
