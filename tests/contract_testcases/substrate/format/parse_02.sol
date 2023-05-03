
        contract c {
            function foo() public {
                string s = "foo{d".format();
            }
        }
// ---- Expect: diagnostics ----
// error: 4:32-35: unexpected format char 'd'
