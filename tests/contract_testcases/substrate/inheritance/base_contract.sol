
        abstract contract base {
            constructor(uint64 a) {}
        }

        contract apex is base {
            constructor() {}
            function foo(uint64 a) virtual external returns (uint64) {
                return a + 102;
            }
        }