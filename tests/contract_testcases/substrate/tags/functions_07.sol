
        contract c {
            /// @inheritdoc
            function foo() public returns (int a, bool b) {}
        }
// ----
// error (39-50): missing contract for tag '@inheritdoc'
