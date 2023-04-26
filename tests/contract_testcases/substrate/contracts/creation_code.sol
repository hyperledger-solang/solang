
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
        
// ----
// warning (85-89): local variable 'code' has been assigned, but never read
// error (255-262): circular reference creating contract 'a'
