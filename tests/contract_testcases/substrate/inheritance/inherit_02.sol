
        contract a is b {
            constructor(int arg1) public {
            }
        }

        contract b is a {
            constructor(int arg1) public {
            }
        }
        
// ----
// error (9-93): missing arguments to base contract 'b' constructor
// error (39-67): missing arguments to contract 'b' constructor
// warning (61-67): 'public': visibility for constructors is ignored
// error (117-118): base 'a' from contract 'b' is cyclic
// warning (155-161): 'public': visibility for constructors is ignored
