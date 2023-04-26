
        contract c {
            /// @return so here we are
            function foo() public returns (int a, bool) {}
        }
// ----
// warning (73-116): function can be declared 'pure'
// warning (108-109): return variable 'a' has never been assigned
