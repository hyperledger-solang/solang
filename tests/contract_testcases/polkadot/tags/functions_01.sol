
        contract c {
            /// @param f
            /// @param g
            function foo(int f) public {}
        }
// ---- Expect: diagnostics ----
// error: 4:24-25: function parameter named 'g' not found
