
        contract a {
            constructor(int arg1) public {
            }
        }

        contract b is a, a {
            constructor(int arg1) public {
            }
        }
        
// ---- Expect: diagnostics ----
// warning: 3:35-41: 'public': visibility for constructors is ignored
// error: 7:9-10:10: missing arguments to base contract 'a' constructor
// error: 7:26-27: contract 'b' duplicate base 'a'
// error: 8:13-41: missing arguments to contract 'a' constructor
// warning: 8:35-41: 'public': visibility for constructors is ignored
