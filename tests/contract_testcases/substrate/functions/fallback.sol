
        contract test {
            int64 result = 102;

            function get() public returns (int64) {
                return result;
            }

            function() external {
                result = 356;
            }
        }