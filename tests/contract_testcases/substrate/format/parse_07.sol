
        contract c {
            function foo() public {
                string s = "{}" "{:x}s".format(1, true);
            }
        }
// ----
// error (108-112): argument must be signed or unsigned integer type
