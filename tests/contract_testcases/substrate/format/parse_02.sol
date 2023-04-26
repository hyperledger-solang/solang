
        contract c {
            function foo() public {
                string s = "foo{d".format();
            }
        }
// ----
// error (89-92): unexpected format char 'd'
