
        contract c {
            /// @return
            function foo() public returns (int a, bool b) {}
        }
// ----
// error (45-45): tag '@return' missing parameter name
