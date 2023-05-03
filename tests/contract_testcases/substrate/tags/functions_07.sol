
        contract c {
            /// @inheritdoc
            function foo() public returns (int a, bool b) {}
        }
// ---- Expect: diagnostics ----
// error: 3:18-4:1: missing contract for tag '@inheritdoc'
