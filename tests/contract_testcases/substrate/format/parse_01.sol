
        contract c {
            function foo() public {
                string s = "foo{".format();
            }
        }
// ---- Expect: diagnostics ----
// error: 4:31-34: missing closing '}'
