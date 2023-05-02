
        contract c {
            function foo() public {
                    string f = "\xたこ";
            }
        }
// ---- Expect: diagnostics ----
// error: 4:33-37: \x escape should be followed by two hex digits
