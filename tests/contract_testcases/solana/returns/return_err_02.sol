
        contract foo {
            uint private val = 0;
            function f() public returns (uint, uint, uint) {
                return (val, val, val);
            }

            function get() public returns (uint, uint) {
                return f();
            }
        }
// ----
// error (247-257): incorrect number of return values, expected 2 but got 3
