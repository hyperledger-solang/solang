
        contract base {
            function foo(uint64 a) override override private returns (uint64) {
                return a + 102;
            }
        }
        