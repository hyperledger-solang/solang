
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
        
// ---- Expect: diagnostics ----
// warning: 4:19-20: local variable 'y' has been assigned, but never read
// error: 10:23-30: circular reference creating contract 'a'
