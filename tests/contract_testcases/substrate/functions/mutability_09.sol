contract test {
            function bar() public payable returns (int64) {
                return 102;
            }
        }
// ---- Expect: diagnostics ----
