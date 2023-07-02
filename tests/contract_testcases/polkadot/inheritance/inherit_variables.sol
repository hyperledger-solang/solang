
        abstract contract b {
            int foo;
        }

        contract c is b {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        
// ---- Expect: diagnostics ----
// warning: 7:13-51: function can be declared 'view'
