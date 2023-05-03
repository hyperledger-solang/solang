
        contract c {
            function test(mapping(int => address) storage x) public {
                //
            }
        }
// ---- Expect: diagnostics ----
// error: 3:51-58: parameter of type 'storage' not allowed public or external functions
