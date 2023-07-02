
        contract c {
            function f() public {
                emit 1 ();
            }
        }
// ---- Expect: diagnostics ----
// error: 4:24-25: unrecognised token '(', expected "++", "--", ".", "[", "case", "default", "leave", "switch", identifier
