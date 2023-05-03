contract test {
            // solc 0.4.25 compiles this to 30.
            function foo() public pure returns (int32) {
                int32 a = b + 3;
                int32 b = a + 7;

                return a * b;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:27-28: 'b' not found
