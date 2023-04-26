
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
// ----
// warning (225-231): 'public': visibility for constructors is ignored
// error (232-233): contract 'b' is not a base contract of 'apex'
