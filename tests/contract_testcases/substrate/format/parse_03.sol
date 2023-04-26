
        contract c {
            function foo() public {
                string s = "foo{:".format();
            }
        }
// ----
// error (88-92): missing format specifier
