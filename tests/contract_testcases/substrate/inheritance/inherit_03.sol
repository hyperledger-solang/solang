
        contract a {
            constructor(int arg1) public {
            }
        }

        contract b is a, a {
            constructor(int arg1) public {
            }
        }
        
// ----
// warning (56-62): 'public': visibility for constructors is ignored
// error (98-185): missing arguments to base contract 'a' constructor
// error (115-116): contract 'b' duplicate base 'a'
// error (131-159): missing arguments to contract 'a' constructor
// warning (153-159): 'public': visibility for constructors is ignored
