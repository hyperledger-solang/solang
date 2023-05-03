
        contract c {
            function test(mapping(int => address) x) public {
                //
            }
        }
// ---- Expect: diagnostics ----
// error: 3:27-50: parameter with mapping type must be of type 'storage'
