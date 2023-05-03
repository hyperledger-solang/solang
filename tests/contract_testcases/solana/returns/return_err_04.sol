
        contract foo {
            function f() public returns (uint, uint, uint) {
                int a = 43;
                return uint(a);
            }
        }
// ---- Expect: diagnostics ----
// error: 5:17-31: incorrect number of return values, expected 3 but got 1
