contract test {
            function foobar(uint32 foo, int16 bar) public returns (bool) {
                foo = bar;
                return false;
            }
        }
// ----
// error (113-116): implicit conversion would change sign from int16 to uint32
