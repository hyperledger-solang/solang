contract test {
            int64 foo = 1844674;

            function bar() public payable returns (int64) {
                return foo;
            }
        }
// ---- Expect: diagnostics ----
