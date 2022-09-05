
        abstract contract base {
            function foo(uint64 a) private returns (uint64) {
                return a + 102;
            }
        }

        contract apex is base {
            function bar() public returns (uint64) {
                return foo({a: 3}) + 3;
            }
        }
        