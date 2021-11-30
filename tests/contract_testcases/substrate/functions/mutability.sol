contract test {
            int64 foo = 1844674;

            function bar() public pure returns (int64) {
                return foo;
            }
        }