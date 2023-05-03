
        contract c {
            function bar() public {
                _;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:17-18: '_' not found
