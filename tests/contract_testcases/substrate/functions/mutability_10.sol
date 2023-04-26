contract test {
            function bar() public view returns (int64) {
                return 102;
            }
        }
// ----
// warning (28-70): function declared 'view' can be declared 'pure'
