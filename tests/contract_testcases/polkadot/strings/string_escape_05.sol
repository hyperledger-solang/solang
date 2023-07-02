
        contract c {
            function foo() public {
                    string f = "\u9kff";
            }
        }
// ---- Expect: diagnostics ----
// error: 4:33-36: \u escape should be followed by four hex digits
