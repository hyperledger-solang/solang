
        abstract contract apex {
            constructor() public apex {

            }
        }
// ---- Expect: diagnostics ----
// warning: 3:27-33: 'public': visibility for constructors is ignored
// error: 3:34-38: contract 'apex' is not a base contract of 'apex'
