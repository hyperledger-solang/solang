
        contract c {
            function foo() public {
                string s = "foo{".format();
            }
        }
// ----
// error (88-91): missing closing '}'
