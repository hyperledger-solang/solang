
        contract foo {
            uint private val = 0;

            function get() public returns (uint, uint) {
                return (val, val, val);
            }
        }
// ----
// error (132-154): incorrect number of return values, expected 2 but got 3
