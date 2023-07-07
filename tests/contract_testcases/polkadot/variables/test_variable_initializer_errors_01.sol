contract test {
            function foo() public pure returns (uint) {
                return 102;
            }
            uint constant y = foo() + 5;
        }
// ---- Expect: diagnostics ----
// error: 5:31-36: cannot call function in constant expression
