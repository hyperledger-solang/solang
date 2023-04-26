
        contract foo {
            uint private val = 0;
            function f() public returns (uint, uint, uint) {
                return (val, val, val);
            }

            function get() public {
                return f();
            }
        }
// ----
// error (226-236): function has no return values
