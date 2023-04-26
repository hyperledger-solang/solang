
        contract foo {
            function f() public returns (uint, uint, uint) {
                int a = 43;
                return uint(a);
            }
        }
// ----
// error (129-143): incorrect number of return values, expected 3 but got 1
