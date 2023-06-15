
        abstract contract base {
            constructor(uint64 a) {}
        }

        contract apex is base {
            constructor() {}
            function foo(uint64 a) virtual external returns (uint64) {
                return a + 102;
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:32-33: function parameter 'a' is unused
// error: 6:9-11:10: missing arguments to base contract 'base' constructor
// error: 7:13-27: missing arguments to contract 'base' constructor
