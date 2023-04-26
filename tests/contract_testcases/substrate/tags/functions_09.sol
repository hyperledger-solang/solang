
        contract c is b {
            /// @param x sadad
            /// @return k is a boolean
            /// @inheritdoc b
            function foo(int x) public pure returns (int a, bool k) {}
        }

        abstract contract b {}
// ----
// warning (156-157): function parameter 'x' has never been read
// warning (184-185): return variable 'a' has never been assigned
// warning (192-193): return variable 'k' has never been assigned
