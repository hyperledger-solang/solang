
        contract c {
            function foo() public {
                    string f = "\x9k";
            }
        }
// ----
// error (90-93): \x escape should be followed by two hex digits
