
        abstract contract b {
            int private foo;
        }

        contract c is b {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 8:24-27: 'foo' not found
