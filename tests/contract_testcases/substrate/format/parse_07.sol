
        contract c {
            function foo() public {
                string s = "{}" "{:x}s".format(1, true);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:51-55: argument must be signed or unsigned integer type
