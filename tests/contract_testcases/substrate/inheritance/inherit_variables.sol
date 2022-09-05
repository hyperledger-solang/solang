
        abstract contract b {
            int foo;
        }

        contract c is b {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        