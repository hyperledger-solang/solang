
        contract c {
            function foo() public {
                string s = "f{{oo}}s".format(true);
            }
        }
// ----
// error (85-108): too many argument for format string
