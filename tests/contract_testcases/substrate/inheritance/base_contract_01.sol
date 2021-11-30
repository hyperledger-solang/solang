
        contract base {
            constructor(uint64 a) public {}
        }

        contract apex is base(true) {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }