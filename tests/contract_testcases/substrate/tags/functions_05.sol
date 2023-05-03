
        contract c {
            /// @return
            function foo() public returns (int a, bool b) {}
        }
// ---- Expect: diagnostics ----
// error: 3:24: tag '@return' missing parameter name
