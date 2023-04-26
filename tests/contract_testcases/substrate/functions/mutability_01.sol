contract test {
            function bar() public constant returns (int64) {
                return 102;
            }
        }
// ----
// warning (28-74): function declared 'view' can be declared 'pure'
// warning (50-58): 'constant' is deprecated. Use 'view' instead
