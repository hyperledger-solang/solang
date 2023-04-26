
        abstract contract b {
            int foo;
        }

        contract c is b {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        
// ----
// warning (101-139): function can be declared 'view'
