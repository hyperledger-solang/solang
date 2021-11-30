
        contract a {
            int public foo;
        }

        contract b is a {
            int public bar;
        }

        contract c is b {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        