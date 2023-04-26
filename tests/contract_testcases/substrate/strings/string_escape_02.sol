
        contract c {
            function foo() public {
                    string f = "\xたこ";
            }
        }
// ----
// error (90-94): \x escape should be followed by two hex digits
