contract test {
            int64 foo = 1844674;

            function bar() public pure returns (int64) {
                return foo;
            }
        }
// ----
// error (123-133): function declared 'pure' but this expression reads from state
