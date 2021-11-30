
        contract a {
            function test() public {
                    bytes code = type(b).creationCode;
            }
        }

        contract b {
                int x;

                function test() public {
                        a f = new a();
                }
        }
        