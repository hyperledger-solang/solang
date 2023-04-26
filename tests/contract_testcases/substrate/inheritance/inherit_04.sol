
        contract a is b {
            constructor(int arg1) public {
            }
        }

        contract b is c {
            constructor(int arg1) public {
            }
        }

        contract c is a {
            constructor(int arg1) public {
            }
        }
        
// ----
// error (9-93): missing arguments to base contract 'b' constructor
// error (9-93): missing arguments to base contract 'c' constructor
// error (39-67): missing arguments to contract 'b' constructor
// warning (61-67): 'public': visibility for constructors is ignored
// error (103-187): missing arguments to base contract 'c' constructor
// error (133-161): missing arguments to contract 'c' constructor
// warning (155-161): 'public': visibility for constructors is ignored
// error (211-212): base 'a' from contract 'c' is cyclic
// warning (249-255): 'public': visibility for constructors is ignored
