contract test {
            function foo() public pure returns (uint) {
                return 102;
            }
            uint constant y = foo() + 5;
        }