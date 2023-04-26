
        abstract contract base {
            constructor(uint64 a) {}
        }

        contract apex is base {
            constructor() {}
            function foo(uint64 a) virtual external returns (uint64) {
                return a + 102;
            }
        }
// ----
// warning (65-66): function parameter 'a' has never been read
// error (90-269): missing arguments to base contract 'base' constructor
// error (126-140): missing arguments to contract 'base' constructor
