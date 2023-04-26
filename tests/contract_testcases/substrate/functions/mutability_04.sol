abstract contract test {
            function bar(int[] storage foo) internal view {
                foo[0] = 102;
            }
        }
// ----
// warning (64-67): function parameter 'foo' has never been read
// error (101-107): function declared 'view' but this expression writes to state
