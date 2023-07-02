
        contract c is b {
            /// @param x sadad
            /// @return k is a boolean
            /// @inheritdoc b
            function foo(int x) public pure returns (int a, bool k) {}
        }

        abstract contract b {}
// ---- Expect: diagnostics ----
// warning: 6:30-31: function parameter 'x' is unused
// warning: 6:58-59: return variable 'a' has never been assigned
// warning: 6:66-67: return variable 'k' has never been assigned
