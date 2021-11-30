contract test {
            function x(int32 arg1) internal {}

            function foo() public {
                function(int32) view a = x;
            }
        }