
        contract c {
            /// @param
            function foo() public {}
        }
// ---- Expect: diagnostics ----
// error: 3:18-23: tag '@param' missing parameter name
