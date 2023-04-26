
        contract c {
            function foo() public {
                string s = "foo";

                s.format();
            }
        }
// ----
// error (109-119): format only allowed on string literals
