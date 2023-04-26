contract test {
            function foobar(int32 foo, uint16 bar) public returns (bool) {
                foo = bar;
                return false;
            }
        }
// ----
// warning (28-88): function can be declared 'pure'
// warning (50-53): function parameter 'foo' has never been read
