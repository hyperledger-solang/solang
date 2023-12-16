
        contract a {
            function test() public {
                    bytes code = type(a).runtimeCode;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:34-53: cannot construct current contract 'a'
