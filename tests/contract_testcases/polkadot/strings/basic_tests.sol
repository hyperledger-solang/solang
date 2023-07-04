
        contract c {
            function foo() public {
                    string f = new string(2);

                    f[0] = 102;
            }
        }
// ---- Expect: diagnostics ----
// error: 6:21-22: array subscript is not permitted on string
