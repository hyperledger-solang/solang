
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
        
// ----
// warning (43-58): storage variable 'foo' has never been used
// warning (183-221): function can be declared 'view'
