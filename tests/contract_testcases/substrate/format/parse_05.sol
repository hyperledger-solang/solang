
        contract c {
            function foo() public {
                string s = "f{{oo}s".format();
            }
        }
// ---- Expect: diagnostics ----
// error: 4:33-37: unmatched '}'
