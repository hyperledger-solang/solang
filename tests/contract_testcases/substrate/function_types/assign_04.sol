contract test {
            function x(int64 arg1) internal returns (bool) {
                return false;
            }

            function foo() public {
                function(int32) returns (bool) a = x;
            }
        }