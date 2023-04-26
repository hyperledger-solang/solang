contract test {
            function foobar(uint foo, int bar) public returns (bool) {
                return (foo < bar);
            }
        }
// ----
// error (111-114): implicit conversion would change sign from uint256 to int256
