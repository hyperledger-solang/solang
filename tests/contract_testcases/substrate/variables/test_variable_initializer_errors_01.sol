contract test {
            function foo() public pure returns (uint) {
                return 102;
            }
            uint constant y = foo() + 5;
        }
// ----
// error (144-149): cannot call function in constant expression
