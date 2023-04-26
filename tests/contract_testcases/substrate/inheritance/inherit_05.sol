
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
        
// ----
// error (9-93): missing arguments to base contract 'b' constructor
// error (9-93): missing arguments to base contract 'c' constructor
// error (9-93): missing arguments to base contract 'd' constructor
// error (39-67): missing arguments to contract 'b' constructor
// warning (61-67): 'public': visibility for constructors is ignored
// error (103-187): missing arguments to base contract 'c' constructor
// error (103-187): missing arguments to base contract 'd' constructor
// error (133-161): missing arguments to contract 'c' constructor
// warning (155-161): 'public': visibility for constructors is ignored
// warning (244-250): 'public': visibility for constructors is ignored
// error (286-373): missing arguments to base contract 'd' constructor
// error (303-304): base 'a' from contract 'c' is cyclic
// error (319-347): missing arguments to contract 'd' constructor
// warning (341-347): 'public': visibility for constructors is ignored
