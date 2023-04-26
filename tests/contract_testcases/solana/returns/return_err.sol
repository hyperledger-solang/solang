
        contract foo {
            uint private val = 0;

            function get() public {
                return val;
            }
        }
// ----
// error (111-121): function has no return values
