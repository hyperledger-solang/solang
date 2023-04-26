
        contract a {
            function x() public {
                b y = new b();
            }
        }

        contract b {
            function x() public {
                a y = new a();
            }
        }
        
// ----
// warning (74-75): local variable 'y' has been assigned, but never read
// error (189-196): circular reference creating contract 'a'
