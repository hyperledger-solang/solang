
        abstract contract base {
            struct s { uint32 f1; }
        }

        abstract contract b {
            struct s { uint32 f1; }
        }

        abstract contract apex is base {
            constructor() public b {

            }
        }
// ---- Expect: diagnostics ----
// warning: 11:27-33: 'public': visibility for constructors is ignored
// error: 11:34-35: contract 'b' is not a base contract of 'apex'
