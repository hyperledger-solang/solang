
        contract c {
            function foo() public {
                string s = "f{{oo}}s".format(true);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:28-51: too many argument for format string
