
        contract c is b {
            /// @param x sadad
            /// @return k is a boolean
            /// @inheritdoc b
            function foo(int x) public pure returns (int a, bool k) {}
        }

        contract b {}