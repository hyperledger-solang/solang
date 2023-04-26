
        abstract contract b {
            int private foo;
        }

        contract c is b {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        
// ----
// error (173-176): 'foo' not found
