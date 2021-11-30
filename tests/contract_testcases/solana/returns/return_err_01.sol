
        contract foo {
            uint private val = 0;

            function get() public returns (uint, uint) {
                return (val, val, val);
            }
        }