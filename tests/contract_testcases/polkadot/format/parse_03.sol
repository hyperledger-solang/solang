
        contract c {
            function foo() public {
                string s = "foo{:".format();
            }
        }
// ---- Expect: diagnostics ----
// error: 4:31-35: missing format specifier
