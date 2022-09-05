
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
        