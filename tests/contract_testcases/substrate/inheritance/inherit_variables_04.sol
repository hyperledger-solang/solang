
        contract a {
            int public constant foo = 0xbffe;
        }

        contract c is a {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        