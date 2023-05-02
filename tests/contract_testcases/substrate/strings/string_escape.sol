
        contract c {
            function foo() public {
                    string f = "\x";
            }
        }
// ---- Expect: diagnostics ----
// error: 4:33-34: \x escape should be followed by two hex digits
