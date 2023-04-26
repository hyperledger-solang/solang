
        contract c {
            function foo() public {
                string s = "foo{:}s".format();
            }
        }
// ----
// error (88-94): missing argument to format
