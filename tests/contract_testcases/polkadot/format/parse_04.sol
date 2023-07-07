
        contract c {
            function foo() public {
                string s = "foo{:}s".format();
            }
        }
// ---- Expect: diagnostics ----
// error: 4:31-37: missing argument to format
