
        contract c {
            function foo() public {
                string s = "f{{oo}s".format();
            }
        }
// ----
// error (90-94): unmatched '}'
