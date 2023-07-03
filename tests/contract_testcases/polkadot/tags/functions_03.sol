
        contract c {
            /// @return so here we are
            function foo() public {}
        }
// ---- Expect: diagnostics ----
// error: 3:18-25: tag '@return' for function with no return values
