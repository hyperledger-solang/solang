
        contract c {
            function test() public returns (mapping(int => address) x) {
                //
            }
        }
// ---- Expect: diagnostics ----
// error: 3:45-68: return type containing mapping must be of type 'storage'
