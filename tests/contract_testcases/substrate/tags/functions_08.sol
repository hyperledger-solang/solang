
        contract c {
            /// @inheritdoc b
            function foo() public returns (int a, bool b) {}
        }
// ---- Expect: diagnostics ----
// error: 3:29-30: base contract 'b' not found in tag '@inheritdoc'
