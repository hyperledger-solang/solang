
        contract c {
            function foo() public {
                    string f = "\uたこ焼き";
            }
        }
// ---- Expect: diagnostics ----
// error: 4:33-37: \u escape should be followed by four hex digits
