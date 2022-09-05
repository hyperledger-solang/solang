
        abstract contract b {
            int private foo;
        }

        contract c is b {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        