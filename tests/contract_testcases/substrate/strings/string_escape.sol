
        contract c {
            function foo() public {
                    string f = "\x";
            }
        }
// ----
// error (90-91): \x escape should be followed by two hex digits
