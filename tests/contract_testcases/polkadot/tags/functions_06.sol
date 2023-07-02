
        contract c {
            /// @return a asda
            /// @return a barf
            function foo() public returns (int a, bool b) {}
        }
// ---- Expect: diagnostics ----
// error: 4:18-25: duplicate tag '@return' for 'a'
