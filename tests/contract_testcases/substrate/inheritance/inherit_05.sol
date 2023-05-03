
        contract a is b {
            constructor(int arg1) public {
            }
        }

        contract b is c {
            constructor(int arg1) public {
            }
        }

        contract d {
            constructor(int arg1) public {
            }
        }

        contract c is d, a {
            constructor(int arg1) public {
            }
        }
        
// ---- Expect: diagnostics ----
// error: 2:9-5:10: missing arguments to base contract 'b' constructor
// error: 2:9-5:10: missing arguments to base contract 'c' constructor
// error: 2:9-5:10: missing arguments to base contract 'd' constructor
// error: 3:13-41: missing arguments to contract 'b' constructor
// warning: 3:35-41: 'public': visibility for constructors is ignored
// error: 7:9-10:10: missing arguments to base contract 'c' constructor
// error: 7:9-10:10: missing arguments to base contract 'd' constructor
// error: 8:13-41: missing arguments to contract 'c' constructor
// warning: 8:35-41: 'public': visibility for constructors is ignored
// warning: 13:35-41: 'public': visibility for constructors is ignored
// error: 17:9-20:10: missing arguments to base contract 'd' constructor
// error: 17:26-27: base 'a' from contract 'c' is cyclic
// error: 18:13-41: missing arguments to contract 'd' constructor
// warning: 18:35-41: 'public': visibility for constructors is ignored
