
        contract a {
            function test() public {
                    bytes code = type(a).runtimeCode;
            }
        }
// ----
// error (92-111): containing our own contract code for 'a' would generate infinite size contract
