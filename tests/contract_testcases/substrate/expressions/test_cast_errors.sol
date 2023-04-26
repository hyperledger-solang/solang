contract test {
            function foo(uint bar) public {
                bool is_nonzero = bar;
            }
        }
// ----
// error (94-97): conversion from uint256 to bool not possible
