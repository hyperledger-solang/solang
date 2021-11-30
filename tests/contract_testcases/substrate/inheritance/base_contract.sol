
        contract base {
            constructor(uint64 a) {}
        }

        contract apex is base {
            constructor() {}
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }