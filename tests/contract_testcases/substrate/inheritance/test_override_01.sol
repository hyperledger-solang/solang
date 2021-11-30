
        contract base {
            function foo(uint64 a) override(bar) private returns (uint64) {
                return a + 102;
            }
        }
        