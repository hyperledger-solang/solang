
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
        
// ---- Expect: diagnostics ----
// warning: 4:27-31: local variable 'code' is unused
// error: 12:31-38: circular reference creating contract 'a'
