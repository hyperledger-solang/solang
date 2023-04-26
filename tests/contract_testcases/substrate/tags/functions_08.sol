
        contract c {
            /// @inheritdoc b
            function foo() public returns (int a, bool b) {}
        }
// ----
// error (50-51): base contract 'b' not found in tag '@inheritdoc'
