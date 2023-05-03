
        contract foo {
            uint private val = 0;

            function get() public returns (uint, uint) {
                return (val, val, val);
            }
        }
// ---- Expect: diagnostics ----
// error: 6:17-39: incorrect number of return values, expected 2 but got 3
