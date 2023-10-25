
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

	    function y() public {
                a y = new a({});
	    }
        }
        
// ---- Expect: diagnostics ----
// warning: 4:19-20: local variable 'y' is unused
// warning: 10:19-20: local variable 'y' is unused
// error: 16:23-30: circular reference creating contract 'a'
// error: 20:23-32: circular reference creating contract 'a'
