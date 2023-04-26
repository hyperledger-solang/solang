
        contract c {
            function foo() public {
                    string f = "\uたこ焼き";
            }
        }
// ----
// error (90-94): \u escape should be followed by four hex digits
