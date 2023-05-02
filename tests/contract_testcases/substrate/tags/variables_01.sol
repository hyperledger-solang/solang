
        abstract contract c is b {
            /// @notice so here we are
            /// @title i figured it out
            /// @inheritdoc b
            int y;
        }

        abstract contract b {}
// ---- Expect: diagnostics ----
// warning: 6:13-18: storage variable 'y' has never been used
