
        contract c {
            function foo() public {
                string s = "foo";

                s.format();
            }
        }
// ---- Expect: diagnostics ----
// error: 6:17-27: format only allowed on string literals
