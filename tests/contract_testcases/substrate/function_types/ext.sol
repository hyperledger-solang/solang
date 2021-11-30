contract test {
            function x(int64 arg1) internal returns (bool) {
                function(int32) external returns (bool) x = foo;
            }

            function foo(int32) public returns (bool) {
                return false;
            }
        }