
        contract c {
            function foo() public {
                    string f = "\u";
            }
        }
// ----
// error (90-91): \u escape should be followed by four hex digits
