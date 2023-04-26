
        contract c {
            function foo() public {
                    string f = "\u9kff";
            }
        }
// ----
// error (90-93): \u escape should be followed by four hex digits
