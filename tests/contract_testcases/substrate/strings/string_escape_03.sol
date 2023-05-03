
        contract c {
            function foo() public {
                    string f = "\u";
            }
        }
// ---- Expect: diagnostics ----
// error: 4:33-34: \u escape should be followed by four hex digits
