
        contract a is b {
            constructor(int arg1) public {
            }
        }

        contract b is a {
            constructor(int arg1) public {
            }
        }
        
// ---- Expect: diagnostics ----
// error: 2:9-5:10: missing arguments to base contract 'b' constructor
// error: 3:13-41: missing arguments to contract 'b' constructor
// warning: 3:35-41: 'public': visibility for constructors is ignored
// error: 7:23-24: base 'a' from contract 'b' is cyclic
// warning: 8:35-41: 'public': visibility for constructors is ignored
