
        contract c {
            function foo() public {
                    string f = "\x9k";
            }
        }
// ---- Expect: diagnostics ----
// error: 4:33-36: \x escape should be followed by two hex digits
