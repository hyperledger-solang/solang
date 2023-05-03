
        contract a {
            int public constant foo = 0xbffe;
        }

        contract c is a {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        
// ---- Expect: diagnostics ----
// warning: 7:13-51: function can be declared 'pure'
