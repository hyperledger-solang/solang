
        contract base {
            constructor(uint64 a) public {}
        }

        contract apex is base(true) {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }
// ----
// warning (59-65): 'public': visibility for constructors is ignored
// error (88-244): missing arguments to base contract 'base' constructor
// error (110-114): conversion from bool to uint64 not possible
