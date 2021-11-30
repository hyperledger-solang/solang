contract test {
            function foobar(uint32 foo, int16 bar) public returns (bool) {
                foo = bar;
                return false;
            }
        }