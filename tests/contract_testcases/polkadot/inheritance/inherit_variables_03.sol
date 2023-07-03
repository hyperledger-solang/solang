
        abstract contract a {
            int private foo;
        }

        abstract contract b is a {
            int public foo;
        }

        contract c is b {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        
// ---- Expect: diagnostics ----
// warning: 3:13-28: storage variable 'foo' has never been used
// warning: 11:13-51: function can be declared 'view'
