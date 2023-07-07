
        contract a {
            function test() public {
                    bytes code = type(a).runtimeCode;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:34-53: containing our own contract code for 'a' would generate infinite size contract
