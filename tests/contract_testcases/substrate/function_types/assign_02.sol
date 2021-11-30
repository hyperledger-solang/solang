contract test {
            function x(int32 arg1) public payable {}

            function foo() public {
                function(int32) a = x;
            }
        }