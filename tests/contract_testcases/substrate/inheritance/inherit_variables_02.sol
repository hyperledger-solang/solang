
        contract a {
            int public foo;
        }

        contract b is a {
            int public bar;
        }

        contract c is b {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        
// ---- Expect: diagnostics ----
// warning: 11:13-51: function can be declared 'view'
