contract test {
            function foobar(int32 foo, uint16 bar) public returns (bool) {
                foo = bar;
                return false;
            }
        }