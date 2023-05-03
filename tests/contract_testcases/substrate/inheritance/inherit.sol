
        contract a is a {
            constructor(int arg1) public {
            }
        }
        
// ---- Expect: diagnostics ----
// error: 2:23-24: contract 'a' cannot have itself as a base contract
// warning: 3:35-41: 'public': visibility for constructors is ignored
