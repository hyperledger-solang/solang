
        contract c {
            /// @return so here we are
            function foo() public returns (int a, bool) {}
        }
// ---- Expect: diagnostics ----
// warning: 4:13-56: function can be declared 'pure'
// warning: 4:48-49: return variable 'a' has never been assigned
