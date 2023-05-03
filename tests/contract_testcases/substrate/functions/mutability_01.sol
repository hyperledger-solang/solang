contract test {
            function bar() public constant returns (int64) {
                return 102;
            }
        }
// ---- Expect: diagnostics ----
// warning: 2:13-59: function declared 'view' can be declared 'pure'
// warning: 2:35-43: 'constant' is deprecated. Use 'view' instead
