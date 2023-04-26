
        contract c {
            function foo() public {
                    string f = new string(2);

                    f[0] = 102;
            }
        }
// ----
// error (125-126): array subscript is not permitted on string
