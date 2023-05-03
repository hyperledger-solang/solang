contract test {
            function bar() public view returns (int64) {
                return 102;
            }
        }
// ---- Expect: diagnostics ----
// warning: 2:13-55: function declared 'view' can be declared 'pure'
