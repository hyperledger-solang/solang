contract test {
            function foobar(uint foo, int bar) public returns (bool) {
                return (foo < bar);
            }
        }
// ---- Expect: diagnostics ----
// error: 3:25-28: implicit conversion would change sign from uint256 to int256
