contract test {
            function x(int32 arg1) internal returns (bool) {
                return false;
            }

            function foo() public {
                function(int32) a = x;
            }
        }