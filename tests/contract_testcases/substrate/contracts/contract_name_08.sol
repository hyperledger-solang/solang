
        contract a {
            function x() public {
                b y = new b();
            }
        }

        contract b {
            function x() public {
                c y = new c();
            }
        }

        contract c {
            function x() public {
                a y = new a();
            }
        }
        
// ----
// warning (74-75): local variable 'y' has been assigned, but never read
// warning (185-186): local variable 'y' has been assigned, but never read
// error (300-307): circular reference creating contract 'a'
